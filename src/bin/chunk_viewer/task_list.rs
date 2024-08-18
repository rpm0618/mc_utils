use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;

pub enum TaskStatus<T> {
    Progress(f32),
    Done(T),
}

pub struct Task<'a, T, C> {
    receiver: Receiver<TaskStatus<T>>,
    callback: Box<dyn FnMut(T, &mut C) + 'a>,
    progress: f32,
}
impl<'a, T, C> Task<'a, T, C> where T: Send + 'static {
    pub fn start(work: impl 'static + Send + FnOnce() -> T, callback: impl 'a + FnMut(T, &mut C)) -> Task<'a, T, C> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = work();
            tx.send(TaskStatus::Done(result)).unwrap();
        });

        Task {
            receiver: rx,
            callback: Box::new(callback),
            progress: 0.0
        }
    }
    
    pub fn start_progress(work: impl 'static + Send + FnOnce(Sender<TaskStatus<T>>) -> T, callback: impl 'a + FnMut(T, &mut C)) -> Task<'a, T, C> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = work(tx.clone());
            tx.send(TaskStatus::Done(result)).unwrap();
        });

        Task {
            receiver: rx,
            callback: Box::new(callback),
            progress: 0.0
        }
    }
}

pub trait Poll<C> {
    fn poll(&mut self, context: &mut C) -> Result<bool, TryRecvError>;
    fn progress(&self) -> f32;
}

impl<'a, T, C> Poll<C> for Task<'a, T, C> where T: Send {
    fn poll(&mut self, context: &mut C) -> Result<bool, TryRecvError> {
        match self.receiver.try_recv() {
            Ok(TaskStatus::Done(result)) => {
                self.progress = 1.0;
                let callback = &mut self.callback;
                callback(result, context);
                Ok(true)
            }
            Ok(TaskStatus::Progress(progress)) => {
                self.progress = progress;
                Ok(false)
            }
            Err(TryRecvError::Empty) => {
                Ok(false)
            }
            Err(TryRecvError::Disconnected) => {
                Err(TryRecvError::Disconnected)
            }
        }
    }

    fn progress(&self) -> f32 {
        self.progress
    }
}

pub struct TaskList<C> {
    tasks: HashMap<String, Box<dyn Poll<C>>>
}
impl<C> TaskList<C> {
    pub fn new() -> TaskList<C> {
        TaskList {
            tasks: HashMap::new()
        }
    }
    
    pub fn add_task(&mut self, name: &str, task: impl Poll<C> + 'static) {
        self.tasks.insert(String::from(name), Box::new(task));
    }

    pub fn poll(&mut self, context: &mut C) {
        let mut to_delete: Vec<String> = Vec::new();
        for (name, task) in &mut self.tasks {
            if task.poll(context).expect("Poll Failed") {
                to_delete.push(name.clone());
            }
        }
        for name in to_delete {
            self.tasks.remove(&name);
        }
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }
    
    pub fn get(&self, name: &str) -> Option<&Box<dyn Poll<C>>> {
        self.tasks.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn Poll<C>>> {
        self.tasks.get_mut(name)
    }
}

impl <'a, C> IntoIterator for &'a TaskList<C> {
    type Item = (&'a String, &'a Box<dyn Poll<C>>);
    type IntoIter = std::collections::hash_map::Iter<'a, String, Box<dyn Poll<C>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.tasks.iter()
    }
}

impl <'a, C> IntoIterator for &'a mut TaskList<C> {
    type Item = (&'a String, &'a mut Box<dyn Poll<C>>);
    type IntoIter = std::collections::hash_map::IterMut<'a, String, Box<dyn Poll<C>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.tasks.iter_mut()
    }
}