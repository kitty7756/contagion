extern crate music;


#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum Music {
    Background,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum TheSound {
    Gunshot,
    Reload,
    PersonInfected,
    ZombieDeath,
}


pub fn load_sound_effects(){
        // Bind the enum variables with the actual mp3 files
        music::bind_music_file(Music::Background, "assets/audio/music/dark_rage.mp3");
        music::bind_sound_file(TheSound::Gunshot, "assets/audio/sfx/gunshot.mp3");
        music::bind_sound_file(TheSound::Reload, "assets/audio/sfx/reload.mp3");
        music::bind_sound_file(TheSound::PersonInfected, "assets/audio/sfx/person_infected.mp3");
        music::bind_sound_file(TheSound::ZombieDeath, "assets/audio/sfx/zombie_dead.mp3");
}

// Plays the background music until the end of the program
pub fn play_background() {
    // Play the Background sound
    music::play_music(&Music::Background, music::Repeat::Forever);
}


// Plays the shotgun sound once every time it is called
pub fn play_shotgun() {
    music::play_sound(&TheSound::Gunshot, music::Repeat::Times(0), 0.5);
}

// Plays the person_infected sound once every time it is called
pub fn play_person_infected() {
    music::play_sound(&TheSound::PersonInfected, music::Repeat::Times(0), 0.5);
}

// Plays the reload sound once every time it is called
pub fn play_reload() {
    music::play_sound(&TheSound::Reload, music::Repeat::Times(0), 0.5);
}

// Plays the dead zombie sound once every time it is called
pub fn play_zombie_dead() {
    music::play_sound(&TheSound::ZombieDeath, music::Repeat::Times(0), 0.5);
}

