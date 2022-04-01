use std::io::{self, BufRead};

pub async fn daemon<T: BufRead>(mut input: T) -> io::Result<()> {
    loop {
        let mut read = String::new();
        match input.read_line(&mut read) {
            Ok(_) => match read.as_str() {
                "quit\n" => break,
                _ => {
                    println!("I saw {}", read);
                }
            },

            Err(e) => panic!("Unable to read line? {}", e),
        }
    }
    Ok(())
}
