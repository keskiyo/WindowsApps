use minisign_verify::{PublicKey, Signature};
use std::env;
use std::fs::File;
use std::io::{Read, Result as IoResult};
use std::process::ExitCode;

const BUFFER_SIZE: usize = 8192;

fn main() -> ExitCode {
    if verify().is_ok() {
        ExitCode::SUCCESS
    } else {
        eprintln!("Updater signature verification failed");
        ExitCode::FAILURE
    }
}

fn verify() -> Result<(), ()> {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let payload_path = arguments.next().ok_or(())?;
    let signature_path = arguments.next().ok_or(())?;
    let public_key_path = arguments.next().ok_or(())?;
    if arguments.next().is_some() {
        return Err(());
    }

    let signature_text = std::fs::read_to_string(signature_path).map_err(|_| ())?;
    let public_key_text = std::fs::read_to_string(public_key_path).map_err(|_| ())?;
    let signature = Signature::decode(&signature_text).map_err(|_| ())?;
    let public_key = PublicKey::decode(&public_key_text).map_err(|_| ())?;
    let mut verifier = public_key.verify_stream(&signature).map_err(|_| ())?;
    let mut payload = File::open(payload_path).map_err(|_| ())?;
    let mut buffer = [0; BUFFER_SIZE];

    loop {
        let count = read_chunk(&mut payload, &mut buffer).map_err(|_| ())?;
        if count == 0 {
            break;
        }
        verifier.update(&buffer[..count]);
    }

    verifier.finalize().map_err(|_| ())
}

fn read_chunk(payload: &mut File, buffer: &mut [u8]) -> IoResult<usize> {
    payload.read(buffer)
}
