use anyhow::Context;
use clap::{ArgGroup, Parser};
use walkdir::WalkDir;

use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use sha1::{Digest, Sha1};

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

type Key = [u8; 32];
type Iv = [u8; 16];

#[derive(Parser)]
#[clap(name = "automachef-transfer")]
#[clap(version = "1.0")]
#[clap(about = "Decrypt, encrypt and transfer platform and user locked Automachef save files.", long_about = None)]
#[clap(group(ArgGroup::new("platforms").args(&["epic", "steam", "twitch"])
    ))]
struct Cli {
    #[clap(
        arg_enum,
        requires_if("encrypt", "platforms"),
        requires_if("transfer", "platforms")
    )]
    action: Action,

    #[clap(parse(from_os_str), value_name = "Save Folder")]
    input: std::path::PathBuf,

    #[clap(long, value_name = "ID")]
    epic: Option<String>,
    #[clap(long, value_name = "ID")]
    steam: Option<String>,
    #[clap(long)]
    twitch: bool,
    #[clap(long)]
    force_overwrite: bool,
}

impl Cli {
    fn get_id(&self) -> Option<String> {
        if let Some(id) = &self.epic {
            Some(id.to_owned())
        } else if let Some(id) = &self.steam {
            Some(id.to_owned())
        } else if self.twitch {
            //base64::encode(b"ajksh54fdhj432h234jh")
            Some("YWprc2g1NGZkaGo0MzJoMjM0amg=".to_owned())
        } else {
            None
        }
    }
}

#[derive(clap::ArgEnum, Clone)]
enum Action {
    Decrypt,
    Encrypt,
    Transfer,
}

struct EncryptedSave {
    data: Vec<u8>,
}

impl EncryptedSave {
    fn new(data: Vec<u8>) -> EncryptedSave {
        EncryptedSave { data }
    }

    fn decrypt(&self, key: &Key) -> anyhow::Result<DecryptedSave> {
        let mut base64_spec = data_encoding::BASE64.specification();
        base64_spec.ignore.push_str(" \t\r\n");
        let base = base64_spec.encoding().unwrap();
        let mut buf = base.decode(self.text())?;

        // guaranteed to fit as base64 > cipher_text >= plaintext
        let plaintext = Aes256CbcDec::new(key.into(), self.iv().into())
            .decrypt_padded_mut::<aes::cipher::block_padding::ZeroPadding>(&mut buf)?;
        let new_len = plaintext.len();
        buf.truncate(new_len);
        Ok(DecryptedSave { text: buf })
    }

    fn _version(&self) -> &[u8] {
        &self.data[0..6]
    }

    fn iv(&self) -> &Iv {
        self.data[6..6 + 16].try_into().unwrap()
    }

    fn text(&self) -> &[u8] {
        &self.data[6 + 16..]
    }
}

impl std::fmt::Display for EncryptedSave {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.data))
    }
}

struct DecryptedSave {
    text: Vec<u8>,
}

impl DecryptedSave {
    fn new(text: Vec<u8>) -> DecryptedSave {
        DecryptedSave { text }
    }

    fn encrypt(self, key: &Key, iv: Option<&Iv>) -> EncryptedSave {
        let iv = iv.unwrap_or(b"0123456789ABCDEF");

        let encrypted_buf =
            Aes256CbcEnc::new(key.into(), iv.into())
                .encrypt_padded_vec_mut::<aes::cipher::block_padding::ZeroPadding>(&self.text);

        let mut data: Vec<u8> =
            Vec::with_capacity(6 + 16 + data_encoding::BASE64.encode_len(encrypted_buf.len()));
        data.extend_from_slice(b"Ver:1\n");
        data.extend_from_slice(iv);
        data.extend_from_slice(data_encoding::BASE64.encode(&encrypted_buf).as_bytes());

        EncryptedSave::new(data)
    }
}

impl std::fmt::Display for DecryptedSave {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.text))
    }
}

struct SaveDir<'a> {
    key: Option<Key>,
    dir: &'a std::path::Path,
}

fn generate_password(id: &str) -> String {
    "hewasindeedandtaforthisimpl".to_owned() + id
}

fn pbkdf1(password: &str) -> Key {
    let salt = "the salty tears provide thy nourishment";
    let iterations = 2;
    let mut last_hash = (password.to_owned() + salt).into_bytes();
    for _ in 0..iterations - 1 {
        last_hash = Sha1::digest(&last_hash).to_vec();
    }
    let mut bytes = Sha1::digest(&last_hash).to_vec();
    let mut ctrl = 1usize;

    while bytes.len() < 32 {
        let mut bytes2 = ctrl.to_string().into_bytes();
        bytes2.append(&mut last_hash);
        bytes.append(&mut Sha1::digest(&mut bytes2).to_vec());
        ctrl += 1;
    }

    bytes[0..32].try_into().unwrap()
}

fn process_entry(
    entry: walkdir::DirEntry,
    action: &Action,
    source: &SaveDir,
    target: &SaveDir,
) -> anyhow::Result<()> {
    if entry.path().is_dir() {
        return anyhow::Ok(());
    }
    let data = std::fs::read(entry.path()).with_context(|| "Failed to read file.")?;

    let converted_data = match action {
        Action::Decrypt => {
            let save = EncryptedSave::new(data);
            save.decrypt(&source.key.expect("Action without source key"))
                .with_context(|| "Failed to decrypt save.")?
                .to_string()
        }
        Action::Encrypt => {
            let save = DecryptedSave::new(data);
            save.encrypt(&target.key.expect("Encryption without target key"), None)
                .to_string()
        }
        Action::Transfer => {
            let save = EncryptedSave::new(data);
            let decrypted = save
                .decrypt(&source.key.expect("Transfer without source key"))
                .with_context(|| "Failed to decrypt save.")?;
            decrypted
                .encrypt(
                    &target.key.expect("Transfer without source key"),
                    Some(save.iv()),
                )
                .to_string()
        }
    };

    let target_file = entry
        .into_path()
        .iter()
        .map(|segment| {
            if segment == source.dir.file_name().expect("Action without source") {
                target.dir.file_name().expect("Action without target")
            } else {
                segment
            }
        })
        .collect::<std::path::PathBuf>();

    if let Some(parent) = &target_file.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dir: {}", parent.display()))?;
    }

    std::fs::write(&target_file, converted_data)
        .with_context(|| format!("Failed to write file: {}", &target_file.display()))
}

fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse();

    let source_id = cli.input.file_name().and_then(|id| id.to_str()).unwrap();
    let source = SaveDir {
        key: Some(pbkdf1(&generate_password(source_id))),
        dir: &cli.input.to_owned(),
    };
    let target_id = cli.get_id();
    let target = SaveDir {
        key: target_id.as_ref().map(|id| pbkdf1(&generate_password(id))),
        dir: {
            let dir_name = match cli.action {
                Action::Decrypt => source_id.to_owned() + "_decrypted",
                Action::Encrypt => target_id.expect("encryption without target"),
                Action::Transfer => target_id.expect("transfer without target"),
            };
            cli.input.set_file_name(dir_name);
            &cli.input
        },
    };

    if !cli.force_overwrite && target.dir.exists() {
        anyhow::bail!("Error: target directory {} already exists. Run again with '--force-overwrite' to overwrite the contents.", target.dir.display())
    }

    for entry in WalkDir::new(&source.dir).into_iter().flatten() {
        println!("{}...", entry.path().display());
        if let Err(error) = process_entry(entry, &cli.action, &source, &target) {
            eprintln!("{}", error);
        }
    }

    Ok(())
}
