use clap::Command;

pub fn command_path<I, S>(root: &Command, args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut current = root;
    let mut path = Vec::new();
    for arg in args.into_iter().skip(1) {
        let arg = arg.as_ref();
        if arg.starts_with('-') {
            continue;
        }
        let Some(subcommand) = current.find_subcommand(arg) else {
            break;
        };
        path.push(subcommand.get_name().to_string());
        current = subcommand;
    }
    path
}
