use crate::event::Event;
use chrono::Duration;
use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::Instant,
};
use timer::{Guard, MessageTimer};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TimerEvent {
    Create(Duration, Option<u32>, u32, bool),
    Trigger(u32),
    Tick,
    Remove(u32),
    Clear(bool),
    Quit,
}

struct Job {
    _guard: Guard,
    count: Option<u32>,
}

struct Schedule {
    main_thread_writer: Sender<Event>,
    jobs: HashMap<u32, Job>,
    core_jobs: HashMap<u32, Job>,
}

impl Schedule {
    fn new(main_thread_writer: Sender<Event>) -> Self {
        Self {
            main_thread_writer,
            jobs: HashMap::new(),
            core_jobs: HashMap::new(),
        }
    }

    fn add_job(&mut self, guard: Guard, count: Option<u32>, callback_id: u32, core: bool) {
        let map = if core {
            &mut self.core_jobs
        } else {
            &mut self.jobs
        };
        map.insert(
            callback_id,
            Job {
                _guard: guard,
                count,
            },
        );
    }

    fn clear_jobs(&mut self, include_core: bool) {
        self.jobs.clear();
        if include_core {
            self.core_jobs.clear();
        }
    }

    fn remove_job(&mut self, callback_id: u32) {
        self.jobs.remove(&callback_id);
    }

    fn run_job(&mut self, callback_id: u32) {
        let opt_job = if self.core_jobs.contains_key(&callback_id) {
            self.core_jobs.get_mut(&callback_id)
        } else {
            self.jobs.get_mut(&callback_id)
        };

        if let Some(job) = opt_job {
            if let Some(count) = job.count {
                if count == 0 {
                    self.main_thread_writer
                        .send(Event::DropTimedEvent(callback_id))
                        .ok();
                    self.jobs.remove(&callback_id);
                } else {
                    job.count.replace(count - 1);
                    self.main_thread_writer
                        .send(Event::TimedEvent(callback_id))
                        .ok();
                }
            } else {
                self.main_thread_writer
                    .send(Event::TimedEvent(callback_id))
                    .ok();
            }
        }
    }
}

pub fn spawn_timer_thread(main_thread_writer: Sender<Event>) -> Sender<TimerEvent> {
    let (sender, receiver): (Sender<TimerEvent>, Receiver<TimerEvent>) = channel();
    let thread_sender = sender.clone();
    thread::Builder::new()
        .name("timer-monitor-thread".to_string())
        .spawn(move || {
            let mut schedule = Schedule::new(main_thread_writer.clone());
            let receiver = receiver;
            let sender = thread_sender;
            let timer = MessageTimer::new(sender);
            let start = Instant::now();
            let _tick_guard =
                timer.schedule_repeating(Duration::milliseconds(100), TimerEvent::Tick);
            loop {
                if let Ok(event) = receiver.recv() {
                    match event {
                        TimerEvent::Create(duration, count, cbid, core) => {
                            let guard =
                                timer.schedule_repeating(duration, TimerEvent::Trigger(cbid));
                            schedule.add_job(guard, count, cbid, core);
                        }
                        TimerEvent::Trigger(cbid) => {
                            schedule.run_job(cbid);
                        }
                        TimerEvent::Tick => {
                            main_thread_writer
                                .send(Event::TimerTick(start.elapsed().as_millis()))
                                .unwrap();
                        }
                        TimerEvent::Remove(cbid) => {
                            schedule.remove_job(cbid);
                        }
                        TimerEvent::Clear(include_core) => {
                            schedule.clear_jobs(include_core);
                        }
                        TimerEvent::Quit => break,
                    }
                }
            }
        })
        .unwrap();
    sender
}

#[cfg(test)]
mod timer_tests {

    use super::{Schedule, TimerEvent};
    use crate::event::Event;
    use chrono::Duration;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use timer::MessageTimer;

    #[test]
    fn test_schedule() {
        let (sender, receiver): (Sender<TimerEvent>, Receiver<TimerEvent>) = channel();
        let (writer, _reader): (Sender<Event>, Receiver<Event>) = channel();
        let timer = MessageTimer::new(sender);
        let mut schedule = Schedule::new(writer);
        let duration = Duration::milliseconds(0);
        let guard = timer.schedule_repeating(duration, TimerEvent::Trigger(1));
        schedule.add_job(guard, Some(1), 1, false);
        if let Ok(event) = receiver.recv() {
            assert_eq!(event, TimerEvent::Trigger(1));
        }
        schedule.run_job(1);
        assert_eq!(schedule.jobs.len(), 1);
        schedule.run_job(1);
        assert!(schedule.jobs.is_empty());

        let guard = timer.schedule_repeating(duration, TimerEvent::Trigger(1));
        schedule.add_job(guard, Some(1), 1, false);
        assert_eq!(schedule.jobs.len(), 1);
        schedule.clear_jobs(true);
        assert!(schedule.jobs.is_empty());
    }
}
