use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    root: PathBuf,
    schema: PathBuf,
    manifest: PathBuf,
    repo_root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "lnf-run-manifest-cli-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temporary fixture directory");

        let schema = root.join("schema.json");
        let manifest = root.join("manifest.json");
        fs::write(
            &schema,
            include_str!("../../../schemas/lnf-e2e-run-manifest-v1.schema.json"),
        )
        .expect("write manifest schema");
        fs::write(
            &manifest,
            include_str!("../../../profiles/lnf-e2e-run-manifest-v1.json"),
        )
        .expect("write manifest fixture");

        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        Self {
            root,
            schema,
            manifest,
            repo_root,
        }
    }

    fn write_manifest(&self, manifest: &Value) {
        fs::write(
            &self.manifest,
            serde_json::to_vec_pretty(manifest).expect("serialize mutated fixture"),
        )
        .expect("write mutated manifest");
    }

    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lnf-run-manifest-verify"))
            .arg(&self.schema)
            .arg(&self.manifest)
            .arg(&self.repo_root)
            .output()
            .expect("run LNF manifest verifier")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn cli_accepts_exact_pins_and_rejects_claim_promotion() {
    let fixture = Fixture::new();
    let first = fixture.run();
    assert!(
        first.status.success(),
        "valid fixture should pass; stdout={} stderr={}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );

    let report: Value = serde_json::from_slice(&first.stdout).expect("JSON validation report");
    assert_eq!(report["valid"], json!(true));
    assert_eq!(report["local_git_blob_sources_verified"], json!(3));
    assert_eq!(report["external_pins_materialized"], json!(false));
    assert_eq!(report["scenario_count"], json!(12));
    assert_eq!(report["execution_conducted"], json!(false));
    assert_eq!(report["measurement_truth_conveyed"], json!(false));

    let mut promoted: Value = serde_json::from_slice(
        include_str!("../../../profiles/lnf-e2e-run-manifest-v1.json").as_bytes(),
    )
    .expect("checked-in manifest parses");
    promoted["claim_ceiling"]["field_observed"] = json!(true);
    fixture.write_manifest(&promoted);

    let second = fixture.run();
    assert_eq!(
        second.status.code(),
        Some(1),
        "claim promotion should be a policy rejection; stdout={} stderr={}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    let report: Value = serde_json::from_slice(&second.stdout).expect("JSON validation report");
    assert_eq!(report["valid"], json!(false));
}
