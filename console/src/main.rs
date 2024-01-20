fn main() {
    let b_opt;
    let m_opt;

    match curseofrust_cli_parser::parse(std::env::args_os()) {
        Ok((b, m)) => {
            b_opt = b;
            m_opt = m;
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
