use community_bandwidth_fairshare_core::{AllocationPolicy, PeerDemand, allocate};
use std::error::Error;
use std::time::Instant;

fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "peer_count,iterations,total_capacity_bytes,allocated_bytes,unallocated_bytes,total_elapsed_ns,ns_per_allocation"
    );
    for (count, iterations) in [(10_usize, 100_u32), (100, 100), (1_000, 20), (4_096, 5)] {
        let capacity = (count as u64) * 10_000;
        let policy = AllocationPolicy {
            total_capacity_bytes: capacity,
            essential_reserve_bytes: capacity / 4,
        };
        let demands: Vec<PeerDemand> = (0..count)
            .map(|index| PeerDemand {
                peer_id: format!("peer-{index:04}"),
                weight: 1 + (index % 5) as u16,
                essential_demand_bytes: 10,
                ordinary_demand_bytes: capacity,
            })
            .collect();

        // Warm once before timing. This is a local algorithm microbenchmark,
        // not a stable latency SLO or a measurement of real network performance.
        let warm = allocate(&policy, &demands)?;
        let started = Instant::now();
        let mut last = warm;
        for _ in 0..iterations {
            last = allocate(&policy, &demands)?;
        }
        let elapsed = started.elapsed().as_nanos();
        let allocated = last.total_essential_granted_bytes + last.total_ordinary_granted_bytes;
        let per_allocation = elapsed / u128::from(iterations);
        println!(
            "{count},{iterations},{capacity},{allocated},{},{elapsed},{per_allocation}",
            last.unallocated_bytes
        );
    }
    Ok(())
}
