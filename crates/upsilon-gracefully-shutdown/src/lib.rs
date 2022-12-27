use std::time::Duration;

use tokio::process::{Child, Command};
#[cfg(all(not(unix), not(windows)))]
compile_error!("Unsupported target platform");

#[cfg(unix)]
pub fn setup_for_graceful_shutdown(cmd: &mut Command) {}

#[cfg(windows)]
pub fn setup_for_graceful_shutdown(cmd: &mut Command) {
    cmd.creation_flags(
        winapi::um::winbase::CREATE_NEW_PROCESS_GROUP | winapi::um::winbase::NORMAL_PRIORITY_CLASS,
    );
}

fn child_proc_id(child: &Child) -> u32 {
    child.id().expect("Cannot get child process id")
}

async fn grace(child: &mut Child, grace_period: Duration) {
    enum GraceEvent {
        ChildExited,
        Timeout,
    }

    let event = tokio::select! {
        _ = child.wait() => GraceEvent::ChildExited,
        _ = tokio::time::sleep(grace_period) => GraceEvent::Timeout,
    };

    match event {
        GraceEvent::ChildExited => {}
        GraceEvent::Timeout => {
            #[cfg(unix)]
            child.kill().await.expect("Failed to kill child");

            #[cfg(windows)]
            {
                println!("Terminating process");
                let success = unsafe {
                    winapi::um::processthreadsapi::TerminateProcess(
                        child.raw_handle().expect("Cannot get raw handle")
                            as winapi::shared::ntdef::HANDLE,
                        1,
                    ) != 0
                };

                if !success {
                    panic!("Failed to kill child");
                }
            }
        }
    }
}

pub async fn gracefully_shutdown(child: &mut Child, grace_period: Duration) {
    #[cfg(unix)]
    let success = unsafe { libc::kill(child_proc_id(child), libc::SIGINT) == 0 };

    #[cfg(windows)]
    let success = unsafe {
        winapi::um::wincon::GenerateConsoleCtrlEvent(
            winapi::um::wincon::CTRL_BREAK_EVENT,
            child_proc_id(child),
        ) != 0
    };

    if !success {
        panic!("Failed to send Ctrl+C signal");
    }

    grace(child, grace_period).await;
}
