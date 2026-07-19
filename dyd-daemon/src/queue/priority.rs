use crate::queue::activity::Priority;

pub fn should_preempt(current: Priority, incoming: Priority) -> bool {
    incoming.rank() > current.rank()
}

pub fn display_duration(p: Priority) -> u64 {
    match p {
        Priority::Critical => 0,
        Priority::High => 6_000,
        Priority::Medium => 4_000,
        Priority::Low => 4_000,
        Priority::Info => 3_000,
    }
}
