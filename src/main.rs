use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use clap::Parser;
use env_logger::Env;

use sysinfo::{System, SystemExt, ProcessExt, PidExt};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use notify_debouncer_full::{DebouncedEvent, new_debouncer, notify};
use notify_debouncer_full::notify::{Event, RecursiveMode, Watcher};

// use notify::{Config, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
#[macro_use]
extern crate log;

/// Simple program that watches a file for changes and sends a SIGHUP signal to a process when the file changes.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Process name that should receive the SIGHUP signal
    #[arg(short, long, env)]
    process_name: String,

    /// Path to the file that should be watched for changes
    #[arg(short, long, env)]
    watch_file: String,

    /// Amount of time to wait once the file was changed before sending the signal to the process
    #[arg(long, env, value_parser = humantime::parse_duration, default_value = "500ms")]
    wait_time: std::time::Duration
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    let process_name = args.process_name;
    let watch_file = args.watch_file;
    let wait_time = args.wait_time;

    info!("Starting filewatch signaler for process: {process_name:?}, watching file {watch_file:?} and duration {wait_time:?}");
    let process_pid = get_mosquitto_pid(&process_name);
    if process_pid.is_none() {
        warn!("Could not find process with name: {}", process_name);
    }

    watch_debouncer(PathBuf::from(&watch_file), move |_event| {
        thread::sleep(args.wait_time);
        info!("File changed. Sending signal.");
        send_sighup_to_process(&process_name);
    }).expect("Error watching file");
}

// fn poll<P: AsRef<Path>, F: Fn(Event)>(path: P, event_handler: F) -> notify::Result<()> {
//     let (tx, rx) = std::sync::mpsc::channel();
//
//     let tx_c = tx.clone();
//     let mut watcher = PollWatcher::new(
//         move |watch_event| {
//             tx_c.send(watch_event).unwrap();
//         },
//         Config::default().with_compare_contents(true).with_poll_interval(std::time::Duration::from_secs(300)),
//     )?;
//
//
//     watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;
//
//     for e in rx {
//         debug!("Watch event {e:?}");
//         match e {
//             Ok(e) => event_handler(e),
//             Err(err) => error!("Error in WatchEvent: {}", err)
//         }
//
//     }
//
//     Ok(())
// }
// fn watch<P: AsRef<Path>, F: Fn(Event)>(path: P, event_handler: F) -> notify::Result<()> {
//     let (tx, rx) = std::sync::mpsc::channel();
//
//     // Automatically select the best implementation for your platform.
//     // You can also access each implementation directly e.g. INotifyWatcher.
//     let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
//
//     // Add a path to be watched. All files and directories at that path and
//     // below will be monitored for changes.
//     watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;
//
//     for res in rx {
//         match res {
//             Ok(event) => event_handler(event),
//             Err(error) => error!("Error: {error:?}"),
//         }
//     }
//
//     Ok(())
// }

fn watch_debouncer<P: AsRef<Path>, F: Fn(Vec<DebouncedEvent>)>(path: P, event_handler: F) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_secs(2), None, tx)?;

    debouncer
        .watcher()
        .watch(path.as_ref(), RecursiveMode::Recursive)
        .unwrap();


    for res in rx {
        match res {
            Ok(event) => event_handler(event),
            Err(error) => error!("Error: {error:?}"),
        }
    }

    Ok(())
}

fn send_sighup_to_process(process_name: &str) {
    match get_mosquitto_pid(process_name) {
        Some(pid) => {
            debug!("Found {} process with pid: {}", process_name, pid);
            send_sighup(pid)
        }
        None => {
            error!("Could not find process with name: {}", process_name);
        }
    }
}

fn send_sighup(process_id: Pid){
    info!("Sending SIGHUP to process");
    kill(process_id, Signal::SIGHUP).unwrap();
}

fn get_mosquitto_pid(process_name: &str) -> Option<Pid> {
    let mut system = System::new_all();
    // First we update all information of our system struct.
    system.refresh_all();

    // Now let's find the Mosquitto process.
    for (pid, process) in system.processes() {
        if process.name().to_lowercase().contains(process_name) {
            debug!("Process: {:?}", process);
            debug!("PID: {}", pid);
            let process_id = pid.as_u32();
            return Some(Pid::from_raw(i32::try_from(process_id).unwrap()));
        }
    }
    return None;
}
