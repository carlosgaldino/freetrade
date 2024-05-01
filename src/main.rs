fn main() {
    let mut args = std::env::args().skip(1);
    let mut input_file = String::new();
    let mut account_name = String::new();
    match args.next() {
        None => exit(1),
        Some(s) => match s.as_str() {
            "-h" | "--help" => exit(0),
            "-i" | "--input" => {
                let Some(file)= args.next() else {
                    return exit(1);
                };
                input_file = file;
                match expect_arg("-a", "--account", &mut args) {
                    Some(account) => account_name = account,
                    None => exit(1),
                }
            }
            "-a" | "--account" => {
                let Some(account)= args.next() else {
                    return exit(1);
                };
                account_name = account;
                match expect_arg("-i", "--input", &mut args) {
                    Some(file) => input_file = file,
                    None => exit(1),
                }
            }
            _ => exit(1),
        },
    }
    println!("input: {}, account: {}", input_file, account_name);
}

fn expect_arg(
    arg_short: &str,
    arg_long: &str,
    args: &mut impl Iterator<Item = String>,
) -> Option<String> {
    let arg = args.next()?;
    match arg.as_str() {
        short if short == arg_short => args.next(),
        long if long == arg_long => args.next(),
        _ => None,
    }
}

fn exit(code: i32) {
    eprintln!("{}", USAGE);
    std::process::exit(code);
}

const USAGE: &str = r#"freetrade - A beancount importer for Freetrade

Usage:
  freetrade --input <FILE> --account <ACCOUNT>
  freetrade --help

Options:
  -i --input   Input file.
  -a --account Account name to use. In the output it will be "Assets:UK:Freetrade:ACCOUNT:SYMBOL".
  -h --help    Print this message.
"#;
