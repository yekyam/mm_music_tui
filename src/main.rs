use clap::{Parser, Subcommand};
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::{env, fs, io};

#[derive(Serialize, Deserialize)]
struct Song {
    artist: String,
    path: String,
    name: String,
}

impl Clone for Song {
    fn clone(&self) -> Song {
        Song {
            artist: self.artist.clone(),
            path: self.path.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Library {
    songs: Vec<Song>,
    streams: Vec<u32>,
}

impl Library {
    // create an empty library
    fn new() -> Library {
        Library {
            songs: vec![],
            streams: vec![],
        }
    }

    fn add(&mut self, song: Song) {
        self.songs.push(song);
    }

    fn play(&self) {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        sink.pause();

        println!("Loading library...");
        for s in &self.songs {
            let f = match File::open(&s.path) {
                Ok(f) => f,
                Err(_) => {
                    println!("\tCouldn't open file: `{}`; skipping", s.path);
                    continue;
                }
            };

            let buf = BufReader::new(f);

            let source = Decoder::new(buf).unwrap();
            sink.append(source);
        }
        println!("Library loaded;");
        sink.play();

        let mut playing = true;
        let mut looping = false;

        loop {
            if sink.len() == 0 {
                break;
            }
            print!(
                "[Playing: {}] {} > ",
                self.songs[self.songs.len() - sink.len()].name,
                env::current_dir()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap()
            );
            io::stdout().flush().unwrap();
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(_) => {}
                Err(e) => {
                    println!("ERROR: couldn't read line from stdin!; {e}");
                }
            }

            let tokens: Vec<&str> = line.split_whitespace().collect();

            if tokens.len() == 0 {
                // println!("In tokens");
                continue;
            }

            match tokens[0] {
                "p" => {
                    playing = !playing;
                    if playing {
                        println!("\tplaying");
                        sink.play();
                    } else {
                        println!("\tnot playing");
                        sink.pause();
                    }
                }
                "s" => {
                    sink.skip_one();
                }
                "l" => {
                    looping = !looping;
                    if looping {
                        println!("\tlooping");
                    } else {
                        println!("\tnot looping");
                    }
                }
                "cd" => {
                    if tokens.len() != 2 {
                        println!("enter a directory");
                    } else {
                        match env::set_current_dir(tokens[1]) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("{e}");
                            }
                        }
                    }
                }
                "q" => {
                    println!("quitting...");
                    exit(0);
                }
                _ => match Command::new(tokens[0]).args(&tokens[1..]).output() {
                    Ok(o) => {
                        println!(
                            "{}{}",
                            String::from_utf8(o.stdout)
                                .expect("WARN: Couldn't convert stdout to string!"),
                            String::from_utf8(o.stderr)
                                .expect("WARN: Couldn't convert stderr to string!")
                        );
                    }
                    Err(e) => {
                        println!("Couldn't execute command!; {e}");
                    }
                },
            }
        }
        if looping {
            self.play();
        }
    }
}

fn get_library(dir_path: &Path) -> Result<Library, ()> {
    let data = match fs::read_to_string(dir_path.join("library.json")) {
        Ok(s) => s,
        Err(e) => {
            println!("WARN: Couldn't read library!; {e}");
            return Err(());
        }
    };

    let lib: Library = match serde_json::from_str(&data) {
        Ok(l) => l,
        Err(e) => {
            println!("ERRORR: Couldn't convert from string to JSON; {e}");
            return Err(());
        }
    };
    Ok(lib)
}

fn get_library_or_create_new_one(dir_path: &Path) -> Result<Library, ()> {
    let res = get_library(dir_path);

    match res {
        Ok(l) => Ok(l),
        Err(_) => {
            println!("Creating new library");
            Ok(Library::new())
        }
    }
}

fn write_library_to_file(library: &Library, dir_path: &Path) -> Result<(), ()> {
    let j = match serde_json::to_string(library) {
        Ok(j) => j,
        Err(e) => {
            println!("FATAL: woahhh couldn't convert library to string; {e}");
            return Err(());
        }
    };

    match fs::write(dir_path.join("library.json"), j) {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("FATAL: woahhh couldn't write library to file; {e}");
            Err(())
        }
    }
}

fn download_song(url: &str, song_name: &str, dir_path: &Path) -> Result<PathBuf, ()> {
    let filename = dir_path.join(song_name.replace(" ", "_") + ".mp3");
    println!("downloading song, this might take a min...");
    let output = {
        Command::new("yt-dlp")
            .arg("--extract-audio")
            .arg("--audio-format")
            .arg("mp3")
            .arg(url)
            .arg("-o")
            .arg(&filename)
            .output()
    };

    match output {
        Ok(s) => {
            println!(
                "{}\n{}",
                String::from_utf8(s.stdout).expect("WARN: Couldn't convert stdout to string!"),
                String::from_utf8(s.stderr).expect("WARN: Couldn't convert stderr to string!")
            );
            if s.status.success() {
                return Ok(filename);
            }
            Err(())
        }
        Err(e) => {
            println!("ERROR: couldn't run command; {e}");
            Err(())
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    // Lists the songs in the library
    List {},

    // Plays the songs in the library
    Play {},

    // Deletes the specified song in the library
    Delete {
        #[arg(long)]
        name: String,
    },

    // Renames the specified song in the library
    Rename {
        #[arg(long)]
        name: String,

        #[arg(long)]
        rename: String,
    },

    // Adds the specified song to the library
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        artist: String,
        #[arg(long)]
        location: String,
    },
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct App {
    #[command(subcommand)]
    commands: Commands,
}

fn main() {
    let args = App::parse();

    // let dir_path = Path::new(".music_library");
    #[allow(deprecated)]
    let dir_path = env::home_dir().unwrap().join(".music_library");

    match std::fs::create_dir_all(&dir_path) {
        Ok(_) => {}
        Err(_) => {
            println!("Couldn't create directories needed!");
        }
    }

    let mut library = match get_library_or_create_new_one(&dir_path) {
        Ok(l) => l,
        Err(_) => {
            println!("ERROR: woahhhh couldn't get library");
            exit(1);
        }
    };

    match &args.commands {
        Commands::List {} => {
            let mut cpy = library.songs.to_vec();
            cpy.sort_by(|a, b| a.artist.cmp(&b.artist));

            println!("{} song(s) in library:", cpy.len());

            for (i, s) in cpy.iter().enumerate() {
                println!("\t{}. {} - {}", i, s.name, s.artist);
            }
        }
        Commands::Play {} => {
            // todo!("do the play features");
            library.play();
        }
        Commands::Delete { name } => {
            println!("Delete from list: {name}");
            let index = library.songs.iter().position(|s| &s.name == name);
            match index {
                Some(i) => {
                    println!("deleting song");
                    library.songs.remove(i);
                }
                None => {
                    println!("Couldn't find song `{name}`, are you sure you typed it correctly??");
                }
            }
        }
        Commands::Add {
            name,
            artist,
            location,
        } => {
            if location.contains("http") {
                match download_song(location, name, &dir_path) {
                    Ok(path) => {
                        let s = Song {
                            artist: artist.clone(),
                            path: path
                                .into_os_string()
                                .into_string()
                                .expect("ERROR: couldn't convert pathh into valid string!"),
                            name: name.clone(),
                        };
                        library.add(s);
                    }
                    Err(_) => {
                        println!("Couldn't download song!");
                    }
                }
            } else {
                todo!("I'm too lazy to implement file copying stuff lol my bad");
            }
            match write_library_to_file(&library, &dir_path) {
                Ok(_) => {
                    println!("Saved library");
                }
                Err(_) => {
                    println!("ERROR: Couldn't save library!");
                }
            }
        }
        Commands::Rename { name, rename } => {
            println!("Rename: {name}\t{rename}");
        }
    };
}
