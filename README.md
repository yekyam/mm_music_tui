# MM_Music_tui

[link to github](https://github.com/yekyam/mm_music_tui) 

<img width="1552" alt="screenshot of TUI in action" src="https://github.com/user-attachments/assets/183bea85-8d21-4b58-b6ba-3f946952b643">


## Features
- Plays music
  - Has basic control features, like play/pause, skip forward/skip back, loop library, repeat song
- Adds music
  - If yt-dlp is installed, will download songs via URL
- Lists music
- Deletes music
- Saves everything to `~/.music_library` for ease of use 

## Usage

### mm_music_tui play 

Plays the songs in the library in a random order. Displays basic controls.

### mm_music_tui add --name (NAME) --artist (ARTIST) --location (LOCATION)

Downloads the song via the location URL, and adds the song with the given name and artist to the library. 

### mm_music_tui delete --name (NAME)

Deletes the given song from the library.

### mm_music_tui rename --name (NAME) --rename (NAME)

Renames the given song in the library to the new name.

### mm_music_tui list 

Outputs the songs in the library, sorted by artist name.

## Contributing

Open an issue, let's discuss.
```
