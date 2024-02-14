use crate::runner::InitContext;
use dialoguer::Input;
use directories::ProjectDirs;

fn init_context() -> InitContext {
    let project_dirs = ProjectDirs::from("", "", "zk_stack_wizard")
        .expect("Unable to initialize project directory");

    todo!()
}
