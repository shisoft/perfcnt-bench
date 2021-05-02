use std::{fs::File, io::{self, LineWriter, Write}, path::Path};

use perfcnt::linux::{CacheId, CacheOpId, CacheOpResultId, PerfCounterBuilderLinux as Builder};
use perfcnt::linux::{HardwareEventType as Hardware, SoftwareEventType as Software};
use perfcnt::{AbstractPerfCounter, PerfCounter};

pub extern crate perfcnt;

pub struct PerfCounters {
    pid: i32,
    counters: Vec<(String, PerfCounter)>,
    results: Vec<(String, u64)>,
}

impl PerfCounters {
    pub fn for_pid(pid: i32) -> Self {
        PerfCounters {
            pid,
            counters: vec![],
            results: vec![],
        }
    }
    pub fn for_this_process() -> Self {
        let pid = unsafe { libc::getpid() };
        println!("Current pid is: {}", pid);
        Self::for_pid(pid)
    }
    pub fn with_software_events(&mut self, events: Vec<Software>) -> &mut Self {
        self.counters.append(
            &mut events
                .into_iter()
                .filter_map(|event| {
                    let name = format!("{:?}", event);
                    match Builder::from_software_event(event)
                        .for_pid(self.pid)
                        .inherit()
                        .on_all_cpus()
                        .exclude_kernel()
                        .exclude_idle()
                        .finish()
                    {
                        Ok(pc) => Some((name, pc)),
                        Err(e) => {
                            println!("Could not create {}, reason '{:?}'", name, e);
                            None
                        }
                    }
                })
                .collect(),
        );
        self
    }
    pub fn with_hardware_events(&mut self, events: Vec<Hardware>) -> &mut Self {
        self.counters.append(
            &mut events
                .into_iter()
                .filter_map(|event| {
                    let name = format!("{:?}", event);
                    match Builder::from_hardware_event(event)
                        .for_pid(self.pid)
                        .inherit()
                        .on_all_cpus()
                        .exclude_kernel()
                        .exclude_idle()
                        .finish()
                    {
                        Ok(pc) => Some((name, pc)),
                        Err(e) => {
                            println!("Could not create {}, reason '{:?}'", name, e);
                            None
                        }
                    }
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
        let name = format!("{:?}_{:?}_{:?}", cache_id, cache_op_id, cache_op_result_id);
        match Builder::from_cache_event(cache_id, cache_op_id, cache_op_result_id)
            .for_pid(self.pid)
            .inherit()
            .on_all_cpus()
            .exclude_kernel()
            .exclude_idle()
            .finish()
        {
            Ok(pc) => {
                self.counters.push((name, pc));
            }
            Err(e) => {
                println!("Could not create {}, reason '{:?}'", name, e);
            }
        }

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
        self.with_all_cache_events_for(&[CacheId::L1D, CacheId::L1I, CacheId::LL, CacheId::NODE])
    }

    pub fn with_all_tlb_cache_events(&mut self) -> &mut Self {
        self.with_all_cache_events_for(&[CacheId::DTLB])
    }

    pub fn with_all_branch_prediction_events(&mut self) -> &mut Self {
        self.with_all_cache_events_for(&[CacheId::BPU])
    }
    pub fn bench<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce() -> R,
    {
        for (c, pc) in &mut self.counters {
            let _ = pc.reset();
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
                        self.results.push((c.to_owned(), num));
                        println!("{}\t{}", c, num)
                    }
                    Err(e) => {
                        println!("Cannot read {}, reason: {}", c, e)
                    }
                }
            }
        }
        res
    }
    pub fn save_result<P: AsRef<Path>>(&mut self, path: P) -> io::Result<&mut Self> {
        if self.results.is_empty() {
            println!("No results to sav");
        } else {
            let file = File::create(path)?;
            let mut file = LineWriter::new(file);
            let head_line = self
                .results
                .iter()
                .map(|(s, _)| s.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let result_line = self
                .results
                .iter()
                .map(|(_, n)| format!("{}", n))
                .collect::<Vec<_>>()
                .join(",");
            file.write(head_line.as_bytes())?;
            file.write(b"\n")?;
            file.write(result_line.as_bytes())?;
            file.flush()?;
        }
        return Ok(self);
    }
}

fn all_cache_ops() -> [CacheOpId; 3] {
    [CacheOpId::Read, CacheOpId::Write, CacheOpId::Prefetch]
}

fn all_cache_res() -> [CacheOpResultId; 2] {
    [CacheOpResultId::Access, CacheOpResultId::Miss]
}

#[cfg(test)]
mod tests {
    use perfcnt::linux::{HardwareEventType, SoftwareEventType};

    use crate::PerfCounters;

    #[test]
    fn it_works() {
        let mut bencher = PerfCounters::for_this_process();
        bencher
            .with_hardware_events(vec![
                HardwareEventType::Instructions,
                HardwareEventType::CPUCycles,
            ])
            .with_software_events(vec![SoftwareEventType::TaskClock])
            .with_all_tlb_cache_events()
            .with_all_mem_cache_events()
            .bench(|| {
                let mut a = 0;
                for i in 0..100 {
                    a += i;
                }
                println!("{}", a);
            });
    }
}
