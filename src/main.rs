use std::{
    thread,
    time::Duration
};

use sdl2::{
    Sdl,
    VideoSubsystem,
    EventPump,
    event::{Event, WindowEvent},
    pixels::Color,
    rect::Rect,
    keyboard::{Scancode, Mod},
    video::{Window, WindowContext},
    render::{Canvas, TextureCreator, Texture},
    ttf::{
        Sdl2TtfContext,
        Font
    }
};


#[derive(Debug, Clone, Copy)]
pub struct Point2<T>
{
    pub x: T,
    pub y: T
}

const FPS: usize = 60;

pub struct Assets<'a>
{
    texture_creator: &'a TextureCreator<WindowContext>
}

impl<'a> Assets<'a>
{
    pub fn new(texture_creator: &'a TextureCreator<WindowContext>) -> Self
    {
        Self{
            texture_creator
        }
    }

    pub fn texture_creator<'b>(&'b self) -> &'a TextureCreator<WindowContext>
    {
        self.texture_creator
    }
}

pub struct WindowHolder
{
    ctx: Sdl,
    video: VideoSubsystem,
    window_size: Point2<u32>,
    canvas: Canvas<Window>
}

impl WindowHolder
{
    pub fn new(window_size: Point2<u32>) -> Self
    {
        let ctx = sdl2::init().unwrap();
        let video = ctx.video().unwrap();

        let window = video.window("base64 decoder", window_size.x, window_size.y)
            .resizable()
            .build()
            .unwrap();

        let canvas = window.into_canvas().build().unwrap();

        Self{
            ctx,
            video,
            window_size,
            canvas
        }
    }

    pub fn texture_creator(&self) -> TextureCreator<WindowContext>
    {
        self.canvas.texture_creator()
    }

    pub fn events(&self) -> EventPump
    {
        self.ctx.event_pump().unwrap()
    }
}

pub struct GameWindow<'a>
{
    window: &'a mut WindowHolder,
    assets: Assets<'a>
}

impl<'a> GameWindow<'a>
{
    pub fn new(window: &'a mut WindowHolder, assets: Assets<'a>) -> Self
    {
        Self{
            window,
            assets
        }
    }

    pub fn canvas(&mut self) -> &mut Canvas<Window>
    {
        &mut self.window.canvas
    }
}

struct Game<'a>
{
    window: GameWindow<'a>,
    ttf_ctx: &'a Sdl2TtfContext,
    font: Font<'a, 'static>,
    text_texture: Option<(Rect, Texture<'a>)>,
    decoded_texture: Option<(Rect, Texture<'a>)>,
    current_text: String,
    decoded_text: String
}

impl<'a> Game<'a>
{
    pub fn new(window: GameWindow<'a>, ttf_ctx: &'a Sdl2TtfContext) -> Self
    {
        let font = Self::create_font(ttf_ctx, 20);

        Self{
            window,
            ttf_ctx,
            font,
            text_texture: None,
            decoded_texture: None,
            current_text: String::new(),
            decoded_text: String::new()
        }
    }

    fn create_text_texture(&self, text: &str) -> Option<(Rect, Texture<'a>)>
    {
        self.font.render(text).blended(Color::RGB(255, 255, 255)).ok().map(|surface|
        {
            let texture_creator = self.window.assets.texture_creator();
            let rect = surface.rect();

            (rect, texture_creator.create_texture_from_surface(surface).unwrap())
        })
    }

    fn create_font(ttf_ctx: &'a Sdl2TtfContext, point: u16) -> Font<'a, 'static>
    {
        ttf_ctx.load_font("font/OpenSans-Regular.ttf", point).unwrap()
    }

    #[allow(dead_code)]
    fn recreate_font(&mut self, point: u16)
    {
        self.font = Self::create_font(self.ttf_ctx, point);
    }

    fn on_event(&mut self, event: Event) -> bool
    {
        match event
        {
            Event::Quit{..} => return false,
            Event::Window{win_event: WindowEvent::Resized(x, y), ..} =>
            {
                self.window.window.window_size = Point2{x: x as u32, y: y as u32};
            },
            Event::TextInput{text, ..} =>
            {
                self.add_text(&text);
            },
            Event::KeyDown{scancode: Some(code), keymod, ..} =>
            {
                match code
                {
                    Scancode::Backspace =>
                    {
                        self.remove_char();
                    },
                    Scancode::Space =>
                    {
                        self.add_text(" ");
                    },
                    Scancode::V if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) =>
                    {
                        match self.window.window.video.clipboard().clipboard_text()
                        {
                            Ok(text) => self.add_text(&text),
                            Err(err) => eprintln!("clipboard error: {err}")
                        }
                    },
                    _ => ()
                }
            },
            _ => ()
        }

        true
    }

    fn add_text(&mut self, s: &str)
    {
        self.current_text += s;

        self.update_text();
    }

    fn remove_char(&mut self)
    {
        self.current_text.pop();

        self.update_text();
    }

    fn update_text(&mut self)
    {
        self.decoded_text = Self::decode_text(&self.current_text);

        self.recreate_textures();
    }

    fn recreate_textures(&mut self)
    {
        self.text_texture = self.create_text_texture(&self.current_text);

        self.decoded_texture = self.create_text_texture(&self.decoded_text);
    }

    fn canvas(&mut self) -> &mut Canvas<Window>
    {
        self.window.canvas()
    }

    fn window_size(&self) -> &Point2<u32>
    {
        &self.window.window.window_size
    }

    fn single_frame(&mut self)
    {
        self.canvas().set_draw_color(Color::RGB(0, 0, 0));
        self.canvas().clear();

        let window_size = *self.window_size();

        let calculate_sizes = |width, height|
        {
            let mut new_width = width;
            let mut new_height = height;

            let ratio = window_size.x as f32 / width as f32;
            if ratio < 1.0
            {
                new_width = window_size.x;
                new_height = (height as f32 * ratio) as u32;
            }

            (new_width, new_height)
        };

        if let Some((text_rect, texture)) = self.text_texture.as_ref()
        {
            let (width, height) = calculate_sizes(text_rect.width(), text_rect.height());

            self.window.canvas().copy(
                texture,
                None,
                Rect::new(0, 0, width, height)
            ).unwrap();
        }

        if let Some((text_rect, texture)) = self.decoded_texture.as_ref()
        {
            let (width, height) = calculate_sizes(text_rect.width(), text_rect.height());

            let y = window_size.y as i32 - height as i32;

            self.window.canvas().copy(
                texture,
                None,
                Rect::new(0, y, width, height)
            ).unwrap();
        }

        self.canvas().present();
    }

    fn decode_text(text: &str) -> String
    {
        let mut values = Self::decode_text_raw(text);

        loop
        {
            if let Some(&last_value) = values.last()
            {
                if last_value == 0
                {
                    values.pop();
                } else
                {
                    break;
                }
            } else
            {
                break;
            }
        }

        let decoded = String::from_utf8_lossy(&values).into_owned();

        decoded.replace(|c: char|
        {
            !(c.is_ascii_graphic() || c == ' ')
        }, &char::REPLACEMENT_CHARACTER.to_string())
    }

    fn decode_text_raw(text: &str) -> Vec<u8>
    {
        let total_bits = text.len() * 6;

        let full_bytes = total_bits / 8;
        let padding_bytes = if (total_bits % 8) == 0 { 0 } else { 1 };

        let mut current_bit = 0;
        let mut values = vec![0; full_bytes + padding_bytes];

        for c in text.chars()
        {
            if let Some(index) = Self::decode_single(c)
            {
                let current_byte = current_bit / 8;

                let bit_remainder = current_bit % 8;
                if bit_remainder > 2
                {
                    // doesnt fit in the current byte cleanly
                    let shift = bit_remainder - 2;
                    values[current_byte] |= index >> shift;

                    let next_shift = 10 - bit_remainder;
                    values[current_byte + 1] |= index << next_shift;
                } else
                {
                    let shift = 2 - bit_remainder;
                    values[current_byte] |= index << shift;
                }
            }

            current_bit += 6;
        }

        values
    }

    fn decode_single(original_char: char) -> Option<u8>
    {
        let c = original_char as u32;

        let value = if (0x41..=0x5a).contains(&c)
        {
            Some(c - 0x41)
        } else if (0x61..=0x7a).contains(&c)
        {
            Some(c - 0x61 + 26)
        } else if (0x30..=0x39).contains(&c)
        {
            Some(c - 0x30 + 52)
        } else if b'+' as u32 == c
        {
            Some(62)
        } else if b'/' as u32 == c
        {
            Some(63)
        } else if b'=' as u32 == c
        {
            Some(0)
        } else
        {
            eprintln!("invalid char: '{original_char}'");

            None
        };

        value.map(|x| x as u8)
    }
}

struct GameWithEvents<'a>
{
    game: Game<'a>,
    events: EventPump
}

impl<'a> GameWithEvents<'a>
{
    pub fn new(game: Game<'a>, events: EventPump) -> Self
    {
        Self{game, events}
    }

    pub fn run(mut self)
    {
        loop
        {
            if !self.single_frame()
            {
                return;
            }

            thread::sleep(Duration::from_millis(1000 / FPS as u64));
        }
    }

    fn single_frame(&mut self) -> bool
    {
        for event in self.events.poll_iter()
        {
            if !self.game.on_event(event)
            {
                return false;
            }
        }

        self.game.single_frame();

        true
    }
}

fn main()
{
    let window_size = Point2{x: 1024, y: 100};

    let ttf_ctx = sdl2::ttf::init().unwrap();

    let mut window_holder = WindowHolder::new(window_size);
    let mut texture_creator = window_holder.texture_creator();

    let assets = Assets::new(&mut texture_creator);

    let events = window_holder.events();
    let window = GameWindow::new(&mut window_holder, assets);

    let game_inner = Game::new(window, &ttf_ctx);
    let game = GameWithEvents::new(game_inner, events);

    game.run();
}
