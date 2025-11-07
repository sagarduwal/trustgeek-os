//! Minimal syscall interface for cooperative kernel.

use crate::scheduler::{Scheduler, TaskCommand};
use crate::timer;

#[repr(u32)]
pub enum SyscallNumber {
    Yield = 0,
    SleepMs = 1,
}

pub enum SyscallResult {
    None,
}

pub fn handle_syscall(num: SyscallNumber, arg0: u32, scheduler: &mut Scheduler) -> SyscallResult {
    match num {
        SyscallNumber::Yield => SyscallResult::None,
        SyscallNumber::SleepMs => {
            let ticks = timer::ms_to_ticks(arg0).max(1);
            scheduler.current_task_sleep(ticks);
            SyscallResult::None
        }
    }
}
