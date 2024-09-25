#[allow(unused_imports)]
use std::{fmt, fs, io::Read};
use std::fmt::format;
use std::io::Write;
use clap::Parser;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use hex;
use hex::ToHex;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
enum Command {
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,
        object_hash: String,
    },
    HashObject {
        #[clap(short = 'w')]
        write: bool,
        file_dir: String,
    },
    #[clap(name = "ls-tree")]
    ListTree {
        #[clap(short = 'n', long = "name-only")]
        name_only: bool,
        tree_hash: String,
    }
}

enum ObjectType {
    Blob,
    Tree
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ObjectType::Blob => write!(f, "blob"),
            ObjectType::Tree => write!(f, "tree"),
        }
    }
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Command::CatFile {
            pretty_print: _,
            object_hash
        } => {
            let (dir, file_name) = object_hash.split_at(2);
            let object_path = format!(".git/objects/{}/{}", dir, file_name);

            let file = fs::File::open(&object_path).unwrap();
            let mut decoder = ZlibDecoder::new(&file);
            let mut contents: String = String::new();
            decoder.read_to_string(&mut contents).unwrap();

            let header_index = contents.find('\0').unwrap();
            let (object_type, object_size) = contents.split_once(' ').unwrap();

            match object_type {
                "blob" => ObjectType::Blob,
                _ => panic!("We are unable to process '{object_type}'.")
            };

            print!("{}", &contents[header_index + 1..])
        },
        Command::ListTree {
            name_only,
            tree_hash,
        } => {
            let (dir, file_name) = tree_hash.split_at(2);
            let object_path = format!(".git/objects/{}/{}", dir, file_name);

            let file = fs::File::open(&object_path).unwrap();
            let mut decoder = ZlibDecoder::new(&file);
            let mut content_bytes = Vec::new();
            decoder.read_to_end(&mut content_bytes).unwrap();

            parse_tree_object(&content_bytes);
            // println!("{:?}", content_bytes);

            // let contents = String::from_utf8(content_bytes).unwrap();
            //
            // let (object_type, _) = contents.split_once(' ').unwrap();
            //
            // match object_type {
            //     "tree" => ObjectType::Tree,
            //     _ => panic!("Unable to process '{object_type}'.")
            // };
            //
            // let lines: Vec<&str> = contents.lines().collect();
            //
            // // let mut tree_objects: Vec<(String, String, String, String)> = Vec::new();
            //
            // for line in lines {
            //     let parts: Vec<&str> = line.split_whitespace().collect();
            //     if parts.len() != 4 { panic!("Invalid tree object") };
            //     let (mode, object_type, hash, path) = (parts[0], parts[1], parts[2], parts[3]);
            //
            //     if name_only {
            //         println!("{}", path);
            //     } else {
            //         println!("{} {} {} {}", mode, object_type, hash, path);
            //     }
            // }
        },
        Command::HashObject {
            write: _,
            file_dir,
        } => {
            let mut file = fs::File::open(&file_dir).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();

            let object_type = ObjectType::Blob;
            let header: String = format!("{} {}\0{}", object_type, contents.len(), contents);

            let mut sha1 = Sha1::new();
            let header_bytes = header.as_bytes();
            sha1.update(header_bytes);
            let sha_hex: String = hex::encode(sha1.finalize());

            let dir = &sha_hex[..2];
            let filename = &sha_hex[2..];

            fs::create_dir_all(format!(".git/objects/{}", dir)).unwrap();
            let file = fs::File::create(format!(".git/objects/{}/{}", dir, filename)).unwrap();

            let bufwriter = std::io::BufWriter::new(&file);
            let mut encoder = ZlibEncoder::new(bufwriter, Compression::default());

            encoder.write_all(&header_bytes).unwrap();

            print!("{}", &sha_hex);
        }
    }
}

fn parse_tree_object(data: &Vec<u8>) {
    let mut i = 0;
    while i < data.len() {
        // Parse the file mode (until the null byte)
        let mode_end = data[i..].iter().position(|&b| b == 0).unwrap() + i;
        let mode = String::from_utf8(Vec::from(&data[i..mode_end])).unwrap();
        i = mode_end + 1; // Skip the null byte

        // Parse the filename (until the next null byte)
        let name_end = data[i..].iter().position(|&b| b == 0).unwrap() + i;
        let filename = String::from_utf8(Vec::from(&data[i..name_end])).unwrap();
        i = name_end + 1; // Skip the null byte

        // Parse the SHA-1 hash (20 bytes)
        let oid = &data[i..i + 20];
        i += 20;

        // Print the mode, filename, and SHA-1 hash
        println!("Mode: {}, Filename: {}, SHA-1: {}", mode, filename, oid.encode_hex::<String>());
    }
}
