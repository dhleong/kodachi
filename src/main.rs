use std::{env, io};

mod connection;
mod transport;

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let mut args = env::args_os().skip(1);
    let uri = args.next();
    if let Some(uri) = uri {
        tokio::spawn(connection::connection(
            uri.to_string_lossy().to_string(),
            5656,
        ))
        .await??;
    } else {
        println!("TODO: daemon mode");
    }
    Ok(())
}
