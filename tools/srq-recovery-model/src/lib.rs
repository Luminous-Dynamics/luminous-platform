//! SRQ-1 executable model for activation-pointer atomicity.
//!
//! This is a filesystem-backed test model, not a bootloader, TUF client, real
//! power-loss simulator, or production update transaction. It uses immutable
//! synthetic generation payloads and an atomically replaced pointer to exercise
//! old-or-new completeness across injected process-interruption checkpoints.

use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};

/// Interruption points around the durable activation-pointer protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationCheckpoint {
    BeforePointerWrite,
    AfterPointerWriteBeforeSync,
    AfterPointerSyncBeforeRename,
    AfterPointerRenameBeforeDirectorySync,
    AfterDirectorySync,
}

/// Outcome when a requested interruption point is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationOutcome {
    Committed,
    InterruptedAt(ActivationCheckpoint),
}

/// A generation that has passed this model's completeness checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveGeneration {
    pub id: String,
    pub payload: Vec<u8>,
}

/// Filesystem-backed model of immutable generations and an atomic current pointer.
#[derive(Debug, Clone)]
pub struct GenerationStore {
    root: PathBuf,
}

impl GenerationStore {
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("generations"))?;
        Ok(Self { root })
    }

    /// Create a deterministic synthetic generation and sync its files before use.
    ///
    /// The payload is intentionally synthetic. The marker validates the exact
    /// bytes expected for this model; it is not a cryptographic artifact proof.
    pub fn prepare_generation(&self, id: &str) -> io::Result<()> {
        validate_id(id)?;
        let dir = self.root.join("generations").join(id);
        fs::create_dir(&dir)?;

        let payload = synthetic_payload(id);
        write_synced(&dir.join("payload.bin"), &payload)?;
        let marker = completeness_marker(id, &payload);
        write_synced(&dir.join("complete.marker"), marker.as_bytes())?;
        sync_directory(&dir)?;
        sync_directory(&self.root.join("generations"))?;
        Ok(())
    }

    /// Activate a complete generation, optionally interrupting at a named boundary.
    ///
    /// The temporary pointer lives beside the active pointer, so rename stays on
    /// the same filesystem. Before rename the prior pointer remains authoritative;
    /// after rename the new pointer references an already-prepared immutable generation.
    pub fn activate(
        &self,
        id: &str,
        interrupt_at: Option<ActivationCheckpoint>,
    ) -> io::Result<ActivationOutcome> {
        validate_id(id)?;
        self.load_generation(id)?;

        if interrupt_at == Some(ActivationCheckpoint::BeforePointerWrite) {
            return Ok(ActivationOutcome::InterruptedAt(
                ActivationCheckpoint::BeforePointerWrite,
            ));
        }

        let temporary_pointer = self.root.join("current.next");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary_pointer)?;
        file.write_all(format!("{id}\n").as_bytes())?;

        if interrupt_at == Some(ActivationCheckpoint::AfterPointerWriteBeforeSync) {
            return Ok(ActivationOutcome::InterruptedAt(
                ActivationCheckpoint::AfterPointerWriteBeforeSync,
            ));
        }

        file.sync_all()?;
        drop(file);

        if interrupt_at == Some(ActivationCheckpoint::AfterPointerSyncBeforeRename) {
            return Ok(ActivationOutcome::InterruptedAt(
                ActivationCheckpoint::AfterPointerSyncBeforeRename,
            ));
        }

        fs::rename(&temporary_pointer, self.root.join("current"))?;

        if interrupt_at == Some(ActivationCheckpoint::AfterPointerRenameBeforeDirectorySync) {
            return Ok(ActivationOutcome::InterruptedAt(
                ActivationCheckpoint::AfterPointerRenameBeforeDirectorySync,
            ));
        }

        sync_directory(&self.root)?;

        if interrupt_at == Some(ActivationCheckpoint::AfterDirectorySync) {
            return Ok(ActivationOutcome::InterruptedAt(
                ActivationCheckpoint::AfterDirectorySync,
            ));
        }

        Ok(ActivationOutcome::Committed)
    }

    /// Read current state fail-closed; never synthesize a fallback generation.
    pub fn active_generation(&self) -> io::Result<ActiveGeneration> {
        let pointer = fs::read_to_string(self.root.join("current"))?;
        let id = pointer.strip_suffix('\n').ok_or_else(|| {
            io::Error::new(ErrorKind::InvalidData, "current pointer must end with one newline")
        })?;
        if id.contains('\n') {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "current pointer must contain exactly one generation identifier",
            ));
        }
        self.load_generation(id)
    }

    fn load_generation(&self, id: &str) -> io::Result<ActiveGeneration> {
        validate_id(id)?;
        let dir = self.root.join("generations").join(id);
        let payload = fs::read(dir.join("payload.bin"))?;
        let marker = fs::read_to_string(dir.join("complete.marker"))?;
        if payload != synthetic_payload(id) || marker != completeness_marker(id, &payload) {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("generation {id} is incomplete or contains mixed model state"),
            ));
        }
        Ok(ActiveGeneration {
            id: id.to_owned(),
            payload,
        })
    }
}

fn validate_id(id: &str) -> io::Result<()> {
    let valid = (3..=64).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-');
    if valid {
        Ok(())
    } else {
        Err(io::Error::new(
            ErrorKind::InvalidInput,
            "generation ID must be 3–64 lowercase ASCII letters, digits, or hyphens",
        ))
    }
}

fn synthetic_payload(id: &str) -> Vec<u8> {
    format!("synthetic-generation-payload:{id}\n").into_bytes()
}

fn completeness_marker(id: &str, payload: &[u8]) -> String {
    format!("generation={id}\npayload_bytes={}\ncomplete=true\n", payload.len())
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new() -> Self {
            loop {
                let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "srq-recovery-model-{}-{id}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create test sandbox: {error}"),
                }
            }
        }

        fn store(&self) -> GenerationStore {
            GenerationStore::open(self.0.clone()).expect("open model store")
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn seeded_store(sandbox: &Sandbox) -> GenerationStore {
        let store = sandbox.store();
        store.prepare_generation("gen-old").expect("prepare old generation");
        store.prepare_generation("gen-next").expect("prepare next generation");
        assert_eq!(
            store.activate("gen-old", None).expect("activate initial generation"),
            ActivationOutcome::Committed
        );
        store
    }

    #[test]
    fn every_interruption_boundary_leaves_old_or_new_complete_generation() {
        let checkpoints = [
            ActivationCheckpoint::BeforePointerWrite,
            ActivationCheckpoint::AfterPointerWriteBeforeSync,
            ActivationCheckpoint::AfterPointerSyncBeforeRename,
            ActivationCheckpoint::AfterPointerRenameBeforeDirectorySync,
            ActivationCheckpoint::AfterDirectorySync,
        ];

        for checkpoint in checkpoints {
            let sandbox = Sandbox::new();
            let store = seeded_store(&sandbox);
            let outcome = store
                .activate("gen-next", Some(checkpoint))
                .expect("injected interruption is an expected outcome");
            assert_eq!(outcome, ActivationOutcome::InterruptedAt(checkpoint));

            // Re-open the store to ensure assertions do not depend on in-memory state.
            let reopened = sandbox.store();
            let active = reopened
                .active_generation()
                .expect("interrupted state must remain readable");
            let expected_id = match checkpoint {
                ActivationCheckpoint::BeforePointerWrite
                | ActivationCheckpoint::AfterPointerWriteBeforeSync
                | ActivationCheckpoint::AfterPointerSyncBeforeRename => "gen-old",
                ActivationCheckpoint::AfterPointerRenameBeforeDirectorySync
                | ActivationCheckpoint::AfterDirectorySync => "gen-next",
            };
            assert_eq!(active.id, expected_id, "checkpoint: {checkpoint:?}");
            assert_eq!(active.payload, synthetic_payload(expected_id));
        }
    }

    #[test]
    fn incomplete_target_is_rejected_without_changing_active_generation() {
        let sandbox = Sandbox::new();
        let store = seeded_store(&sandbox);
        let incomplete = store.root.join("generations").join("gen-bad");
        fs::create_dir(&incomplete).expect("create incomplete generation fixture");
        fs::write(
            incomplete.join("payload.bin"),
            b"synthetic-generation-payload:gen-bad\n",
        )
        .expect("write payload without completion marker");

        let error = store
            .activate("gen-bad", None)
            .expect_err("incomplete generation must not activate");
        assert_eq!(error.kind(), ErrorKind::NotFound);
        assert_eq!(
            store.active_generation().expect("old active generation").id,
            "gen-old"
        );
    }

    #[test]
    fn malformed_or_truncated_current_pointer_fails_closed() {
        let sandbox = Sandbox::new();
        let store = seeded_store(&sandbox);
        fs::write(store.root.join("current"), "gen-ne").expect("truncate pointer");
        let error = store
            .active_generation()
            .expect_err("malformed pointer must not choose a fallback");
        assert_eq!(error.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn path_like_generation_ids_are_rejected() {
        let sandbox = Sandbox::new();
        let store = sandbox.store();
        assert!(store.prepare_generation("../escape").is_err());
        assert!(store.prepare_generation("Uppercase").is_err());
    }

    #[test]
    fn interrupted_temporary_pointer_is_never_treated_as_active() {
        let sandbox = Sandbox::new();
        let store = seeded_store(&sandbox);
        assert_eq!(
            store
                .activate(
                    "gen-next",
                    Some(ActivationCheckpoint::AfterPointerSyncBeforeRename),
                )
                .expect("inject interruption"),
            ActivationOutcome::InterruptedAt(
                ActivationCheckpoint::AfterPointerSyncBeforeRename
            )
        );
        assert_eq!(
            store.active_generation().expect("old pointer remains").id,
            "gen-old"
        );
        assert!(sandbox.0.join("current.next").exists());
    }
}
