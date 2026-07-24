use ed25519_dalek::SigningKey;
use rand_core::OsRng;

use crate::{cli::KeygenArgs, key_file};

pub(super) fn run(args: KeygenArgs) -> Result<(), String> {
    let key = SigningKey::generate(&mut OsRng);
    let public_path = key_file::write_pair(&args.output, &key)?;
    println!("private key: {}", args.output.display());
    println!("public key:  {}", public_path.display());
    println!(
        "public hex:  {}",
        hex::encode(key.verifying_key().as_bytes())
    );
    println!(
        "key id:      {}",
        hex::encode(sg_format::key_id(&key.verifying_key()))
    );
    println!("Keep the private key outside the repository and launcher installation.");
    Ok(())
}
