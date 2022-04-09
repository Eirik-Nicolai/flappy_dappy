use std::{collections::HashMap, default, fmt};

use ggez::graphics;

struct Animation
{
    sheet_pos: (f32,f32),
    curr_index: i8,
    animation_length: i8,
    is_playing: bool,
    loop_animation: bool,
    frame_delay: u8,
    curr_frame_delay_tick: u8
}

impl Animation
{
    fn new(sheet_pos: (f32,f32), animation_length: i8, loops: bool, frame_delay: u8) -> Animation
    {
        Animation { 
            sheet_pos,
            curr_index: 0,
            animation_length,
            is_playing: false,
            loop_animation: loops,
            frame_delay,
            curr_frame_delay_tick: frame_delay
        }
    }

    fn step(&mut self)
    {
        if self.is_playing
        {
            if self.curr_frame_delay_tick == 0
            {        
                if self.curr_index == self.animation_length
                {
                    self.curr_index = 0;
                    if !self.loop_animation
                    {
                        self.is_playing = false;
                    }
                }
                else if self.curr_index < self.animation_length
                {
                    self.curr_index += 1;
                }
                self.curr_frame_delay_tick = self.frame_delay;
            }
            else 
            {
                self.curr_frame_delay_tick -= 1;    
            }
        }
    }

    fn stop(&mut self)
    {
        self.is_playing = false;
        self.curr_index = 0;
    }
}

impl fmt::Display for Animation
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pos: {:?}, curr_index: {}, animation_length: {}, is_playing:{}", 
            self.sheet_pos, self.curr_index, self.animation_length, self.is_playing)
    }
}

pub struct Spritesheet
{
    pub sheet: graphics::Image,
    img_size: (f32,f32),
    pub img_scale: f32,

    animations: HashMap<String, Animation>,
    pub sprite_size: (f32,f32),

    step_len: f32,
    has_animation_in_progress: bool
}

impl Spritesheet
{
    pub fn new(sheet: graphics::Image, scale:f32, sprite_size:(f32,f32), step_len: f32) -> Spritesheet
    {
        let img_size = (sheet.width() as f32, sheet.height() as f32);

        Spritesheet {
            sheet,
            img_size,
            img_scale: scale,
            animations: HashMap::new(),
            sprite_size,
            step_len,
            has_animation_in_progress: false
        }
    }

    pub fn add_animation(&mut self,title: &str, sheet_pos: (f32,f32), animation_length: i8, frame_delay: u8)
    {
        self.animations.insert(title.to_string(), 
            Animation::new(sheet_pos, animation_length, true, frame_delay)
        );
    }
    
    pub fn add_animation_looping(&mut self,title: &str, sheet_pos: (f32,f32), animation_length: i8, loops:bool, frame_delay: u8)
    {
        self.animations.insert(title.to_string(), Animation::new(sheet_pos, animation_length, loops, frame_delay));
    }

    pub fn start_animation(&mut self, title: &str) -> bool
    {
        match self.animations.get_mut(title)
        {
            Some(anim) => {
                anim.is_playing = true;
                true
            },
            None => false
        }
    }

    pub fn tick(&mut self)
    {
        for (_, anim) in &mut self.animations
        {
            if anim.is_playing
            {
                anim.step();
            }
        }
    }

    pub fn draw(&self) -> graphics::Rect
    {
        for (_, anim) in &self.animations
        {
            if anim.is_playing
            {
                let corrected_size = (self.sprite_size.0 as f32/ self.img_size.0,self.sprite_size.1 as f32/ self.img_size.1 as f32);
                let corrected_pos = (
                    (anim.sheet_pos.0 + self.step_len * anim.curr_index as f32) / self.img_size.0,
                    (anim.sheet_pos.1) / self.img_size.1,
                );
                return graphics::Rect::new(
                    corrected_pos.0,
                    corrected_pos.1,
                    corrected_size.0,
                    corrected_size.1
                );
            }
        }

        let anim = &self.animations["idle"];

        let corrected_size = (self.sprite_size.0 as f32/ self.img_size.0,self.sprite_size.1 as f32/ self.img_size.1 as f32);
        let corrected_pos = (
            (anim.sheet_pos.0 + self.step_len * anim.curr_index as f32) / self.img_size.0,
            (anim.sheet_pos.1 + self.step_len * anim.curr_index as f32) / self.img_size.1,
        );
        graphics::Rect::new(
            corrected_pos.0,
            corrected_pos.1,
            corrected_size.0,
            corrected_size.1
        )
    }
}

impl fmt::Debug for Spritesheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut anims = Vec::new();
        for (str, anim) in &self.animations
        {
            anims.push(format!("id:{},animation:{}",str,anim));
        }
        write!(f, "Spritesheet [({:?}), has_animation: {}]", anims,self.has_animation_in_progress)
    }
}