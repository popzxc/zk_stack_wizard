use xshell::{cmd, Shell};

pub(super) fn check_prerequisites(shell: &Shell) {
    prerequisite(shell, "docker");
    prerequisite(shell, "docker-compose");
}

fn prerequisite(shell: &Shell, name: &str) {
    let success = cmd!(shell, "which {name}").quiet().output().is_ok();
    if !success {
        println!("Prerequisite check has failed");
        println!("❌ {name} is not available");
        println!("Make sure that {name} is available on your machine");
        println!("Exiting");
        std::process::exit(1);
    }
}
