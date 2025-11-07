//! Cooperative task scheduler.
//!
//! Tasks implement the [`Task`] trait and are polled cooperatively. Each call to
//! [`Scheduler::run_ready`] polls every task that is ready to run based on the
//! system tick counter maintained by `timer`.

use core::cmp::Ordering;

use heapless::Vec;

use esp_println::println;

use crate::{
    stack::{TaskStack, DEFAULT_STACK_SIZE},
    timer,
};

/// Maximum number of tasks supported by the kernel.
pub const MAX_TASKS: usize = 8;

/// Unique identifier assigned to each spawned task.
pub type TaskId = u32;

/// Possible errors when spawning or managing tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    /// Maximum task count reached.
    NoCapacity,
    /// Allocation failed when reserving per-task stack.
    OutOfMemory,
}

/// Result of polling a task.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskCommand {
    /// Continue running on the next scheduler cycle (no delay).
    Continue,
    /// Sleep for the given number of ticks.
    SleepTicks(u32),
    /// Sleep for the given number of milliseconds.
    SleepMs(u32),
    /// Task has completed and will be removed from the scheduler.
    Finished,
}

/// Task priority used for cooperative ordering (higher runs earlier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// Context passed to each task when it is polled.
#[allow(dead_code)]
pub struct TaskContext {
    /// The unique identifier for the task being polled.
    pub id: TaskId,
    /// Current system tick when the task was polled.
    pub current_ticks: u32,
}

/// Trait implemented by cooperative tasks.
pub trait Task {
    /// Human-readable task name (for diagnostics).
    fn name(&self) -> &'static str;

    /// Task priority (defaults to [`TaskPriority::Normal`]).
    fn priority(&self) -> TaskPriority {
        TaskPriority::default()
    }

    /// Requested stack size in bytes.
    fn stack_size(&self) -> usize {
        DEFAULT_STACK_SIZE
    }

    /// Poll the task once.
    fn poll(&mut self, ctx: &mut TaskContext) -> TaskCommand;
}

struct TaskSlot {
    id: TaskId,
    task: &'static mut dyn Task,
    priority: TaskPriority,
    next_run_tick: u32,
    finished: bool,
    stack: TaskStack,
}

impl TaskSlot {
    fn new(id: TaskId, task: &'static mut dyn Task, now: u32) -> Result<Self, SchedulerError> {
        let priority = task.priority();
        let stack = TaskStack::new(task.stack_size()).ok_or(SchedulerError::OutOfMemory)?;
        Ok(Self {
            id,
            task,
            priority,
            next_run_tick: now,
            finished: false,
            stack,
        })
    }
}

/// Cooperative multitasking scheduler.
pub struct Scheduler {
    tasks: Vec<TaskSlot, MAX_TASKS>,
    next_id: TaskId,
}

impl Scheduler {
    /// Create an empty scheduler.
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    /// Register a new task with the scheduler.
    pub fn spawn(&mut self, task: &'static mut dyn Task) -> Result<TaskId, SchedulerError> {
        if self.tasks.is_full() {
            return Err(SchedulerError::NoCapacity);
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let slot = TaskSlot::new(id, task, timer::get_ticks())?;
        self.tasks
            .push(slot)
            .map_err(|_| SchedulerError::NoCapacity)?;
        Ok(id)
    }

    /// Poll all tasks that are ready to run at the current tick.
    pub fn run_ready(&mut self) {
        let now = timer::get_ticks();

        // Sort tasks so higher priority ones run first.
        self.tasks.sort_unstable_by(|a, b| match b.priority.cmp(&a.priority) {
            Ordering::Equal => a.next_run_tick.cmp(&b.next_run_tick),
            other => other,
        });

        for slot in self.tasks.iter_mut() {
            if slot.finished {
                continue;
            }

            if !slot.stack.verify() {
                println!("Stack guard tripped for task {}", slot.task.name());
                slot.finished = true;
                continue;
            }

            if now < slot.next_run_tick {
                continue;
            }

            let mut ctx = TaskContext {
                id: slot.id,
                current_ticks: now,
            };

            match slot.task.poll(&mut ctx) {
                TaskCommand::Continue => {
                    slot.next_run_tick = now;
                }
                TaskCommand::SleepTicks(ticks) => {
                    slot.next_run_tick = now.wrapping_add(ticks.max(1));
                }
                TaskCommand::SleepMs(ms) => {
                    let ticks = timer::ms_to_ticks(ms).max(1);
                    slot.next_run_tick = now.wrapping_add(ticks);
                }
                TaskCommand::Finished => {
                    slot.finished = true;
                }
            }

            if !slot.stack.verify() {
                println!("Stack guard tripped after polling task {}", slot.task.name());
                slot.finished = true;
            }
        }
    }

    /// Remove finished tasks (optional housekeeping).
    #[allow(dead_code)]
    pub fn reap_finished(&mut self) {
        self.tasks.retain(|slot| !slot.finished);
    }

    /// Total number of tasks currently managed by the scheduler.
    #[allow(dead_code)]
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}
