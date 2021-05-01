use perfcnt::linux::{CacheId, CacheOpId, CacheOpResultId, PerfCounterBuilderLinux as Builder};
use perfcnt::linux::{HardwareEventType as Hardware, SoftwareEventType as Software};
use perfcnt::{AbstractPerfCounter, PerfCounter};
use std::process;

pub struct PerfCounters {
    pid: i32,
    counters: Vec<(String, PerfCounter)>,
}

impl PerfCounters {
    pub fn for_pid(pid: i32) -> Self {
        PerfCounters {
            pid, counters: vec![]
        }
    }
    pub fn for_this_process() -> Self {
        Self::for_pid(process::id() as i32)
    }
    pub fn with_software_events(&mut self, events: Vec<Software>) -> &mut Self {
        self.counters.append(
            &mut events
                .into_iter()
                .map(|event| {
                    (
                        format!("{:?}", event),
                        Builder::from_software_event(event)
                        .on_all_cpus()
                        .for_pid(self.pid)
                        .finish()
                        .expect("Could not create counter")
                    )
                })
                .collect(),
        );
        self
    }
    pub fn with_hardware_events(&mut self, events: Vec<Hardware>) -> &mut Self {
        self.counters.append(
            &mut events
                .into_iter()
                .map(|event| {
                    (
                        format!("{:?}", event),
                        Builder::from_hardware_event(event)
                        .on_all_cpus()
                        .for_pid(self.pid)
                        .finish()
                        .expect("Could not create counter")
                    )
                })
                .collect(),
        );
        self
    }
    pub fn with_cache_event(
        &mut self,
        cache_id: CacheId,
        cache_op_id: CacheOpId,
        cache_op_result_id: CacheOpResultId,
    ) -> &mut Self {
        self.counters.push(
            (
                format!("{:?}_{:?}_{:?}", cache_id, cache_op_id, cache_op_result_id),
                Builder::from_cache_event(cache_id, cache_op_id, cache_op_result_id)
                .on_all_cpus()
                .for_pid(self.pid)
                .finish()
                .expect("Could not create counter"),
            )
        );
        self
    }
    pub fn with_all_cache_events_for(&mut self, events: &[CacheId]) -> &mut Self {
        let cache_ops = all_cache_ops();
        let cache_res = all_cache_res();
        for id in events.iter() {
            for op in cache_ops.iter() {
                for res in cache_res.iter() {
                    self.with_cache_event(*id, *op, *res);
                }
            }
        }
        self
    }
    pub fn with_all_mem_cache_events(&mut self) -> &mut Self {
        self.with_all_cache_events_for(&[
            CacheId::L1D,
            CacheId::L1I,
            CacheId::LL,
            CacheId::NODE
        ])
    }

    pub fn with_all_tlb_cache_events(&mut self) -> &mut Self {
        self.with_all_cache_events_for(&[
            CacheId::DTLB
        ])
    }

    pub fn with_all_branch_prediction_events(&mut self) -> &mut Self {
        self.with_all_cache_events_for(&[
            CacheId::BPU
        ])
    }
    pub fn bench<F, R>(&mut self, func: F) -> R 
        where F: Fn() -> R
    {
        for (c, pc) in &mut self.counters {
            if let Err(e) = pc.start() {
                println!("Cannot start {}, reason: {}", c, e);
            }
        }
        let res = func();
        for (c, pc) in &mut self.counters {
            if let Err(e) = pc.stop() {
                println!("Cannot stop {}, reason: {}", c, e);
            } else {
                match pc.read() {
                    Ok(num) => {
                        println!("{}\t{}", c, num)
                    }
                    Err(e) => {
                        println!("Cannot read {}, reason: {}", c, e)
                    }
                }
                if let Err(e) = pc.reset() {
                    println!("Cannot reset {}, reason: {}", c, e);
                }
            }
        }
        res
    }
}

fn all_cache_ops() -> [CacheOpId; 3] {
    [
        CacheOpId::Read,
        CacheOpId::Write,
        CacheOpId::Prefetch
    ]
}

fn all_cache_res() -> [CacheOpResultId; 2] {
    [
        CacheOpResultId::Access,
        CacheOpResultId::Miss
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
