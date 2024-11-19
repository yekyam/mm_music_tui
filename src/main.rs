use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};

use rand::seq::SliceRandom;
use rand::thread_rng;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::sync::mpsc::channel;
use std::{env, fs, io};
use std::{fs::File, time::Duration};
use std::{
    io::BufReader,
    sync::{Arc, Mutex},
};

#[derive(Serialize, Deserialize, Default)]
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

#[derive(Default)]
pub struct RApp {
    current_playing: Song,
    exit: bool,
    looping: bool,
    song_repeat: bool,
    is_playing: bool,
}

enum Actions {
    Playing,
    Paused,
    Skip,
    Back,
    Loop,
    RepeatSong,
    NoRepeat,
    NoLoop,
}

fn make_source(path: &str) -> Option<Decoder<BufReader<File>>> {
    let f = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            return None;
        }
    };

    let buf = BufReader::new(f);

    Some(Decoder::new(buf).unwrap())
}

impl RApp {
    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn run(&mut self, songs: &[Song], terminal: &mut DefaultTerminal) -> io::Result<()> {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        sink.play();

        self.is_playing = true;
        self.looping = false;

        let mut songs = songs.to_owned();
        songs.shuffle(&mut thread_rng());

        let (sender, reciever) = channel();
        let i_original = Arc::new(Mutex::new(0));

        // thrread vars
        let t_songs = songs.clone();
        let t_i = i_original.clone();
        std::thread::spawn(move || {

            let mut looping = false;
            let mut repeat_song = false;

            let source = make_source(&t_songs[*t_i.lock().unwrap()].path).unwrap();
            sink.append(source);
            sink.play();

            loop {
                if let Ok(action) =  reciever.recv_timeout(Duration::from_millis(5)) {
                    match action {
                        Actions::Back => {
                            let mut i = t_i.lock().unwrap();

                            if *i != 0 {
                                *i -= 1;
                                let source = make_source(&t_songs[*i].path).unwrap();
                                sink.append(source);
                                sink.skip_one();
                            }
                        }
                        Actions::Skip => {
                            let mut i = t_i.lock().unwrap();

                            if looping && (*i + 1) == t_songs.len() {
                                *i = 0;
                            } else if *i < t_songs.len() - 1 {
                                *i += 1;
                            }
                            let source = make_source(&t_songs[*i].path).unwrap();
                            sink.append(source);
                            sink.skip_one();
                        }
                        Actions::RepeatSong => repeat_song = true,
                        Actions::NoRepeat => repeat_song = false,
                        Actions::Loop => looping = true,
                        Actions::NoLoop => looping = false,
                        Actions::Paused => sink.pause(),
                        Actions::Playing => sink.play(),
                    }
                };

                if sink.len() == 0 {

                    let mut i = t_i.lock().unwrap();

                    if repeat_song {

                        let source = make_source(&t_songs[*i].path).unwrap();
                        sink.append(source);
                        sink.play();
                        continue;

                    }

                    *i += 1;

                    if *i == t_songs.len() {
                        if looping {
                            *i = 0;
                        } else {
                            ratatui::restore();
                            exit(0);
                        }
                    }
                    let source = make_source(&t_songs[*i].path).unwrap();
                    sink.append(source);
                    sink.play();
                }
            }
        });

        while !self.exit {
            self.current_playing = songs[*i_original.lock().unwrap()].clone();
            terminal.draw(|frame| self.draw(frame))?;
            // self.handle_events()?;
            if event::poll(Duration::from_millis(5))? {
                match event::read()? {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        match key_event.code {
                            KeyCode::Char('q') => self.exit(),
                            KeyCode::Left => {
                                sender.send(Actions::Back).unwrap();
                            }
                            KeyCode::Right => {
                                sender.send(Actions::Skip).unwrap();
                            }
                            KeyCode::Char(' ') => {
                                self.is_playing = !self.is_playing;
                                if self.is_playing {
                                    sender.send(Actions::Playing).unwrap();
                                } else {
                                    sender.send(Actions::Paused).unwrap();
                                }
                            }
                            KeyCode::Char('l') => {
                                self.looping = !self.looping;
                                if self.looping {
                                    sender.send(Actions::Loop).unwrap();
                                } else {
                                    sender.send(Actions::NoLoop).unwrap();
                                }
                            }
                            KeyCode::Char('r') => {
                                self.song_repeat = !self.song_repeat;
                                if self.song_repeat {
                                    sender.send(Actions::RepeatSong).unwrap();
                                } else {
                                    sender.send(Actions::NoRepeat).unwrap();
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &RApp {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let title = Line::from(" Music TUI ".bold());
        let instructions = Line::from(vec![
            " Pause ".into(),
            "<Space>".blue().bold(),
            " Skip ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q>".blue().bold(),
            " Loop ".into(),
            "<L>".blue().bold(),
            " Repeat Song ".into(),
            "<R>".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let l1 = Line::from(vec![
            "Playing: ".into(),
            self.current_playing.name.clone().into(),
        ]);

        let l2 = Line::from(vec![
            "By: ".into(),
            self.current_playing.artist.clone().yellow(),
        ]);

        let l3 = Line::from(vec![
            "Looping: ".into(),
            if self.song_repeat {
                "SONG".yellow()
            } else if self.looping {
                "LIB".yellow()
            } else {
                "OFF".yellow()
            },
        ]);

        let l4 = Line::from(vec![if self.is_playing {
            "Playing".yellow()
        } else {
            "Paused".yellow()
        }]);


        let song_text = Text::from(vec![l1, l2, l3, l4]);

        // let chunks = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints([Constraint::Length(3), Constraint::Min(1)])
        //     .split(area);

        // Gauge::default()
        //     .block(Block::bordered().title("progress"))
        //     .gauge_style(Style::new().white().on_black().italic())
        //     .ratio(1.0 / self.prog.as_secs() as f64)
        //     .render(chunks[0], buf);

        Paragraph::new(song_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
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
            let mut terminal = ratatui::init();
            match RApp::default().run(&library.songs, &mut terminal) {
                Ok(_) => {}
                Err(e) => {
                    println!("error running stuff!; {e}");
                }
            }
            ratatui::restore();

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
