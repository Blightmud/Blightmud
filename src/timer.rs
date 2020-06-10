use crate::event::Event;
use chrono::Duration;
use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};
use timer::{Guard, MessageTimer};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TimerEvent {
    Create(Duration, Option<u32>, u32),
    Trigger(u32),
    Clear,
    Quit,
}

struct Job {
    _guard: Guard,
    count: Option<u32>,
}

struct Schedule {
    main_thread_writer: Sender<Event>,
    jobs: HashMap<u32, Job>,
}

impl Schedule {
    fn new(main_thread_writer: Sender<Event>) -> Self {
        Self {
            main_thread_writer,
            jobs: HashMap::new(),
        }
    }

    fn add_job(&mut self, guard: Guard, count: Option<u32>, callback_id: u32) {
        self.jobs.insert(
            callback_id,
            Job {
                _guard: guard,
                count,
            },
        );
    }

    fn clear_jobs(&mut self) {
        self.jobs.clear();
    }

    fn run_job(&mut self, callback_id: u32) {
        if let Some(job) = self.jobs.get_mut(&callback_id) {
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
    thread::spawn(move || {
        let mut schedule = Schedule::new(main_thread_writer);
        let receiver = receiver;
        let sender = thread_sender;
        let timer = MessageTimer::new(sender);
        loop {
            if let Ok(event) = receiver.recv() {
                match event {
                    TimerEvent::Create(duration, count, cbid) => {
                        let guard = timer.schedule_repeating(duration, TimerEvent::Trigger(cbid));
                        schedule.add_job(guard, count, cbid);
                    }
                    TimerEvent::Trigger(cbid) => {
                        schedule.run_job(cbid);
                    }
                    TimerEvent::Clear => {
                        schedule.clear_jobs();
                    }
                    TimerEvent::Quit => break,
                }
            }
        }
    });
    sender
}
