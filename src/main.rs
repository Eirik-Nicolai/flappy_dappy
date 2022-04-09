mod animation;

use std::time::Duration;
use std::{path};

use ggez::graphics::spritebatch::SpriteBatch;
use glam::*;
use rand::*;

use ggez::*;
use ggez::conf::{WindowSetup, WindowMode, FullscreenType};
use ggez::event::MouseButton;
use ggez::graphics::*;
use specs::*;

type Point2 = Vec2;

// DEBUGGING 
const SHOW_HITBOXES:bool = false;
const RUN_SYS_MOVEMENT:bool = false;
const RUN_SYS_COLLISION:bool = false;
const RUN_SYS_OBSTACLES:bool = false;


const OBST_AMOUNT:u8 = 3;

const WINDOW_H:f32 = 1100.0;
const WINDOW_W:f32 = 1000.0;

const SQUARE_SIZE:f32 = 80.0;

const OBSTACLE_TIGTHFACTOR:f32 = 220.0;

const RNG_LOW:f32 = 0.2;
const RNG_HIGH:f32 = 1.8;

const OBST_SPEED:f32 = -3.5;
const BIRD_FLAP:f32 = 3.5;
const GRAVITY:f32 = 10.0;

#[derive(Clone, Copy, Default)]
struct Delta(Duration);

#[derive(Clone, Copy, Default)]
struct Gravity(f32);

#[derive(Clone, Copy, Default)]
struct Score(u8);

#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Rect
{
    pos_x: f32,
    pos_y: f32,
    size_x: f32,
    size_y: f32
}
impl PartialEq for Rect {
    fn eq(&self, other: &Self) -> bool {
        self.pos_x == other.pos_x
        && self.pos_y == other.pos_y
        && self.size_x == other.size_x
        && self.size_y == other.size_y
    }
}
impl Eq for Rect {}

#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Velocity
{
    x: f32,
    y: f32
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Animation
{
    spritesheet: animation::Spritesheet,
}

#[derive(Component,Default)]
#[storage(NullStorage)]
struct Controllable;

#[derive(Component,Default)]
#[storage(NullStorage)]
struct Dirty;


#[derive(Component,Default)]
#[storage(NullStorage)]
struct Collision;

#[derive(Component,Default)]
#[storage(VecStorage)]
struct Obstacle(u8);

#[derive(Component,Default)]
#[storage(VecStorage)]
struct IsGameover(bool);

enum State
{
    Menu,
    Playing,
    GameOver
}

struct GameState
{
    state: State,
    difficulty: u8, // TODO increase with score? i guess

    obst_sheet: graphics::Image,

    ecs: World,
    movement_sys: MovementSystem,
    obstacle_sys: ObstacleSysten,
    collision_sys: CollisionSystem,
    score_sys: ScoreSystem
}
impl GameState
{
    fn new(_ctx: &mut Context, player_spritesheet: animation::Spritesheet) -> GameResult<GameState>
    {
        let mut world = World::new();

        world.insert(Delta(Duration::from_nanos(0)));
        world.insert(Gravity(GRAVITY));
        world.insert(Score(0));
        world.insert(IsGameover(false));
        
        world.register::<Rect>();
        world.register::<Dirty>();
        world.register::<Obstacle>();
        world.register::<Velocity>();
        world.register::<Animation>();
        world.register::<Collision>();
        world.register::<Controllable>();
        
        world
            .create_entity()
            .with(Rect{ pos_x: WINDOW_W/3.0 - SQUARE_SIZE, pos_y: WINDOW_H/3.0 - SQUARE_SIZE, 
                size_x: SQUARE_SIZE, size_y: SQUARE_SIZE})
            .with(Velocity { x: 0.0, y: 0.0 })
            .with(Collision)
            .with(Controllable)
            .with(Animation {
                spritesheet: player_spritesheet
            })
            .build();
        
        world
            .create_entity()
            .with(Rect{ pos_x: -1.0, pos_y: -1.0,
                size_x: WINDOW_W + 1.0, size_y: SQUARE_SIZE/2.0})
            .with(Collision)
            .build();
            
        world
            .create_entity()
            .with(Rect{ pos_x: -1.0, pos_y: WINDOW_H - (SQUARE_SIZE/2.0),
                size_x: WINDOW_W + 1.0, size_y: SQUARE_SIZE/2.0})
            .with(Collision)
            .build();

        create_obstacles(&mut world, OBST_AMOUNT);
        
        
        let gs = GameState {
            state: State::Menu,
            difficulty: 0,
            obst_sheet: graphics::Image::new(_ctx, "/obst.png").unwrap(),
            ecs: world,
            movement_sys: MovementSystem,
            obstacle_sys: ObstacleSysten,
            collision_sys: CollisionSystem,
            score_sys: ScoreSystem
        };
        Ok(gs)
    }

    fn reset_game(&mut self)
    {
        let mut rect  = self.ecs.write_storage::<Rect>();
        let mut dirty = self.ecs.write_storage::<Dirty>();
        let obst = self.ecs.read_storage::<Obstacle>();
        let mut velo  = self.ecs.write_storage::<Velocity>();
        let controllable  = self.ecs.write_storage::<Controllable>();

        let entities = self.ecs.entities();

        for (r, vel, _) in (&mut rect, &mut velo, &controllable).join()
        {
            *r = Rect{ pos_x: WINDOW_W/3.0 - SQUARE_SIZE, pos_y: WINDOW_H/3.0 - SQUARE_SIZE, size_x: SQUARE_SIZE, size_y: SQUARE_SIZE};
            *vel = Velocity { x: 0.0, y: 0.0 };
        }
        
        let mut rng = rand::thread_rng();
        for (ent, r, vel, obs) in (&entities, &mut rect, &mut velo, &obst).join()
        {
            let height_from_ceiling = (WINDOW_H/2.0) * rng.gen_range::<f32, f32, f32>(RNG_LOW, RNG_HIGH);
            *r = Rect{ pos_x: WINDOW_W+50.0, pos_y: 0.0, size_x: 2.0*SQUARE_SIZE/3.0, size_y: height_from_ceiling - (OBSTACLE_TIGTHFACTOR/2.0)};
            let v = if obs.0 == 0
            {
                OBST_SPEED
            }
            else 
            {
                0.0
            };
            
            *vel = Velocity { x: v, y: 0.0 };
            if let None = dirty.remove(ent)
            {
                // nothing happens...
            }
        }

        let mut score = self.ecs.write_resource::<Score>();
        *score = Score(0);
        let mut is_gameover = self.ecs.write_resource::<IsGameover>();
        *is_gameover = IsGameover(false);

        self.state = State::Playing;
    }
}

impl ggez::event::EventHandler<GameError> for GameState
{
    fn update(&mut self, ctx: &mut Context) -> GameResult
    {
        let delta = timer::delta(ctx);
        
        if self.ecs.read_resource::<IsGameover>().0
        {
            self.state = State::GameOver;
        }

        match self.state
        {
            State::Playing => {    
                {   // we do these in their own scope as the systems need &mut btw
                    // UPDATE GAME STATE
                    let mut input_state = self.ecs.write_resource::<Delta>();
                    *input_state = Delta(delta);
                    
                    // UPDATE PHYSICS / FALLING
                    
                    let grav = self.ecs.read_resource::<Gravity>();
                    let mut velo  = self.ecs.write_storage::<Velocity>();
                    let control = self.ecs.read_storage::<Controllable>();
            
                    for (vel, _) in (&mut velo, &control).join()
                    {
                        let dt = delta.as_secs_f32();
                        vel.y += grav.0 * dt;
                    }

                    // UPDATE ANIMATIONS

                    let mut animation  = self.ecs.write_storage::<Animation>();
                    for anim in (&mut animation).join()
                    {
                        anim.spritesheet.tick();
                    }
                }
                if RUN_SYS_MOVEMENT
                {
                    self.movement_sys.run_now(&self.ecs);
                }
                if RUN_SYS_COLLISION
                {
                    self.obstacle_sys.run_now(&self.ecs);
                }
                if RUN_SYS_OBSTACLES
                {
                    self.collision_sys.run_now(&self.ecs);
                }
                
                self.score_sys.run_now(&self.ecs);
            
            }
            _ => {}
        }

        self.ecs.maintain();

        Ok(())
    }
    // TODO all text could be one draw call but it doesn't matter at this scale
    fn draw(&mut self, ctx: &mut Context) -> GameResult
    {
        graphics::clear(ctx, [1.0;4].into());

        let rect = self.ecs.read_storage::<Rect>();

        //  ---------- PLAYER -------------

        let velo  = self.ecs.read_storage::<Velocity>();
        let animation  = self.ecs.read_storage::<Animation>();
        for (anim, r, v) in (&animation, &rect, &velo).join()
        {
            // bird sprite
            let drawparams = graphics::DrawParam::new()
                .dest([
                    r.pos_x+anim.spritesheet.sprite_size.0*1.0,
                    r.pos_y+anim.spritesheet.sprite_size.1*1.5
                ])
                .scale(Point2::new(anim.spritesheet.img_scale, anim.spritesheet.img_scale))
                .offset(Point2::new(0.5, 0.5))
                .rotation(translate_player_rotation(&v.y))
                .src(anim.spritesheet.draw());
            graphics::draw(ctx, &anim.spritesheet.sheet, drawparams)?;
            
            // score text
            let score = self.ecs.read_resource::<Score>();
            let font = graphics::Font::new(ctx, "/font.ttf")?;
            let text = graphics::Text::new(
                (score.0.to_string(),
                font, 
                60.0
            ));
            graphics::draw(ctx, 
                &text, 
                graphics::DrawParam::new()
                        .dest(Point2::new(
                            r.pos_x + anim.spritesheet.sprite_size.0/2.0,
                            r.pos_y + anim.spritesheet.sprite_size.1 + 20.0
                        ))
                        .color(Color::from((0, 0, 0, 255)))
                )?;
        }

        //  --------------------------------

        //  --------- OBSTACLES ------------

        let obst= self.ecs.read_storage::<Obstacle>();
        let mut batch = SpriteBatch::new(self.obst_sheet.clone());

        // TODO this could be done not here like animations
        for (_, r, _) in (&obst, &rect, &velo).join()
        {
            // sausage sprites
            // basically figure out a bunch of positional translations from the sprite images
            // TODO this should be frontloaded
            let img_size = (self.obst_sheet.width() as f32,self.obst_sheet.height() as f32);
            let corrected_size = (
                510.0 / img_size.0,
                110.0 / img_size.1 as f32
            );
            let corrected_pos = (
                0.0 / img_size.0,
                20.0 / img_size.1,
            );
            let corrected_size_head = (
                110.0 / img_size.0,
                110.0 / img_size.1 as f32
            );
            let corrected_pos_head = (
                0.0 / img_size.0,
                150.0 / img_size.1,
            );

            // are we displaying a downwards or an upwards saus
            let (y_offs, nega) = if r.pos_y == 0.0
            {
                (r.size_y, -1.0)
            }
            else 
            {
                (r.pos_y, 1.0)
            };

            let drawparams_head = graphics::DrawParam::new()
                .dest([
                    r.pos_x+(SQUARE_SIZE/3.0)+2.0*nega,
                    y_offs + 50.0 * nega
                ])
                .offset(Point2::new(0.5, 0.5))
                .rotation(-1.5708 * nega)
                .src(graphics::Rect::new(
                    corrected_pos_head.0,
                    corrected_pos_head.1,
                    corrected_size_head.0,
                    corrected_size_head.1
            ));

            let drawparams = graphics::DrawParam::new()
                .dest([
                    r.pos_x+(SQUARE_SIZE/3.0),
                    y_offs + ((WINDOW_H/4.0) + (85.0)) * nega
                ])
                .offset(Point2::new(0.5, 0.5))
                .rotation(-1.5708 * nega)
                .src(graphics::Rect::new(
                    corrected_pos.0,
                    corrected_pos.1,
                    corrected_size.0,
                    corrected_size.1
            ));

            // sausage head logic
            if (r.pos_y == 0.0 && r.size_y > 400.0) 
                || (r.pos_y != 0.0 && r.pos_y < 450.0)
            {
                if r.pos_y == 0.0
                {
                    let drawparams = graphics::DrawParam::new()
                        .dest([
                            r.pos_x+(SQUARE_SIZE/3.0),
                            r.pos_y - (WINDOW_H/2.0 - r.size_y)
                        ])
                        .offset(Point2::new(0.5, 0.5))
                        .rotation(-1.5708 * nega)
                        .src(graphics::Rect::new(
                            corrected_pos.0,
                            corrected_pos.1,
                            corrected_size.0,
                            corrected_size.1
                    ));
                    batch.add(drawparams);
                }
                else
                {
                    let drawparams = graphics::DrawParam::new()
                        .dest([
                            r.pos_x+(SQUARE_SIZE/3.0),
                            r.pos_y + r.size_y
                        ])
                        .offset(Point2::new(0.5, 0.5))
                        .rotation(-1.5708 * nega)
                        .src(graphics::Rect::new(
                            corrected_pos.0,
                            corrected_pos.1,
                            corrected_size.0,
                            corrected_size.1
                    ));
                    batch.add(drawparams);
                };
            }
                
            batch.add(drawparams);
            batch.add(drawparams_head);
        }
        
        graphics::draw(ctx, &batch, DrawParam::default())?;
        
        //  --------------------------------

        //  ---------- HITBOXES -------------
        
        if SHOW_HITBOXES
        {
            let mut mb = MeshBuilder::new();
            for r in (&rect).join()
            {
                if let Err(e) = mb.polygon(DrawMode::fill(),
                    &[
                        Vec2::new(r.pos_x, r.pos_y),
                        Vec2::new(r.pos_x+r.size_x, r.pos_y),
                        Vec2::new(r.pos_x+r.size_x, r.pos_y+r.size_y),
                        Vec2::new(r.pos_x, r.pos_y+r.size_y)],
                    Color::new(1.0, 0.0, 1.0, 1.0),
                )
                {
                    println!("Couldn't create mesh on error {e}");
                };
            }
            let mesh = mb.build(ctx).unwrap();
            
            graphics::draw(
                ctx, 
                &mesh,
                graphics::DrawParam::new()
                    .color(Color::from_rgb(200, 200, 200))
                    .dest([0.0,0.0])
            )?;
        }
        
        //  -------------------------------

        
        //  --------- MENU THINGS -----------

        match self.state
        {
            State::Menu => {
                let font = graphics::Font::new(ctx, "/font.ttf")?;
                let text = graphics::Text::new(
                    ("WELCOME TO FLAPPY DAPPY.\n MOYSE CLICK MOVES U UP\n\nTRY TO AVOID THE SAUSAGES",
                    font, 
                    60.0
                ));
                graphics::draw(ctx, 
                &text, 
                graphics::DrawParam::new()
                        .dest(Point2::new(
                            WINDOW_W/2.0-text.dimensions(ctx).w/2.0,
                            WINDOW_H/2.0
                        ))
                        .color(Color::from((0, 0, 0, 255)))
                )?;
            },
            State::GameOver => {
                let score = self.ecs.read_resource::<Score>().0;
                let font = graphics::Font::new(ctx, "/font.ttf")?;
                let text = graphics::Text::new(
                    (format!("UR TRASH \n\nUR SCORE WAS {}\n\n\nCLICK MOUSE TO RESET", score),
                    font, 
                    60.0
                ));
                
                graphics::draw(ctx, 
                &text, 
                graphics::DrawParam::new()
                        .dest(Point2::new(
                            WINDOW_W/2.0-text.dimensions(ctx).w/2.0,
                            WINDOW_H/2.0
                        ))
                        .color(Color::from((0, 0, 0, 255)))
                )?;
            }
            _ => {}
        }
        
        //  -------------------------------
        
        graphics::present(ctx)?;
        Ok(())
    }

    fn key_down_event(
        &mut self, 
        ctx: &mut ggez::Context,
        key: event::KeyCode, 
        _kmod: ggez::event::KeyMods, 
        _repeat: bool)
    {
        if !_repeat
        {
            match key 
            {
                event::KeyCode::Escape => { //exit game
                    ctx.continuing = false;
                },
                _ => {}
            }
        }
    }
    
    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32
    )
    {
        match button
        {
            MouseButton::Left => 
            {
                match self.state 
                {
                    State::Menu => {
                        self.state = State::Playing;
                    },
                    State::Playing => {
                        let mut velo = self.ecs.write_storage::<Velocity>();
                        let mut animation = self.ecs.write_storage::<Animation>();
                        let control = self.ecs.read_storage::<Controllable>();
                        for (vel, _, anim) in (&mut velo, &control, &mut animation).join()
                        {
                            if vel.y > BIRD_FLAP-(BIRD_FLAP*0.3)// we do an extra chec kto double the jump vel
                            {                                   // so it feels a bit better to control
                                vel.y -= BIRD_FLAP*2.0;
                            }
                            else 
                            {
                                vel.y -= BIRD_FLAP;
                            }
                            anim.spritesheet.start_animation("flap");
                        }
                    },
                    State::GameOver => {
                        self.reset_game();
                    }
                }
            },
            _ => {}
        }
    }
}

/// Translate the player velocity to radians of rotation
/// Basically do (velocity*1.57)/rotational_factor
/// 1.57 being 90* in radians
fn translate_player_rotation(vel: &f32) -> f32
{
    let limit = 13.0;
    if vel.abs() < limit
    {
        return (vel*1.57)/limit
    }
    //todo there's gotta be a better waty
    if vel < &0.0
    {
        return -1.57
    }
    1.57
}

fn main()
{
    let mut cb = ContextBuilder::new(
        "flappydappy","NIC")
        .window_mode(WindowMode {
            width: WINDOW_W,
            height: WINDOW_H,
            borderless: false,
            fullscreen_type: FullscreenType::Windowed,
            min_width: 0.0,
            max_width: 0.0,
            min_height: 0.0,
            max_height: 0.0,
            maximized: false,
            resizable: false,
            visible: true,
            resize_on_scale_factor_change: false,
        })
        .window_setup(WindowSetup::default().title("FLAPPY DAPPY"));

    let manifest_dir = "C:/Users/eirik/OneDrive/Desktop/CODE/RUST/GGEZ/projects/flappydappy";
    let mut path = path::PathBuf::from(manifest_dir);
    path.push("assets");
    cb = cb.add_resource_path(path);
    
    let (mut ctx, event_loop) = cb.build().unwrap();
    
    let player_spritesheet_img = graphics::Image::new(&mut ctx, "/bird.png").unwrap();
    let mut player_spritesheet = animation::Spritesheet::new(
        player_spritesheet_img,
        3.0,
        (32.0,25.0),
        35.0
    );

    player_spritesheet.add_animation("idle",(145.0,145.0), 0, 0);
    player_spritesheet.add_animation_looping("flap",(5.0,144.0), 3, false, 1);
    
    let state = GameState::new(&mut ctx, player_spritesheet).unwrap();

    event::run(ctx, event_loop, state);
}


struct ObstacleSysten;
impl<'a> System<'a> for ObstacleSysten
{
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Rect>,
        WriteStorage<'a, Dirty>,
        ReadStorage<'a, Obstacle>,
        WriteStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData)
    {
        let (entity, mut rect, 
            mut dirty, obst, 
            mut velocity) 
            = data;
        
        let mut reload_obstacles = None;

        // has an obstacle gone outside the screen bounds?

        'reload: for (e_outer, r, obs_outer, _) in (&entity, &rect, &obst, &velocity).join()
        {
            if r.pos_x + r.size_x + 50.0 < 0.0 // check for offset of when to move
            {   
                let id = obs_outer.0;
                for (e_inner, obs,r_inner,  _) in (&entity, &obst, &rect, &velocity).join()
                {
                    if id == obs.0 && r != r_inner
                    {
                        if r.pos_y < r_inner.pos_y
                        {
                            reload_obstacles = Some((e_outer,e_inner));
                        }
                        else
                        {
                            reload_obstacles = Some((e_inner,e_outer));
                        }
                        dirty.remove(e_inner);
                        dirty.remove(e_outer);
                            
                        break 'reload;
                    }
                }
            }
        }

        if let Some((e_upper,e_lower)) = reload_obstacles
        {   // if yes, move it to the start and remove velocity component so it can wait to be spawned
            let mut rng = rand::thread_rng();
            let height_from_ceiling = (WINDOW_H/2.0) * rng.gen_range::<f32, f32, f32>(RNG_LOW, RNG_HIGH);
            
            let mut r_upper = rect.get_mut(e_upper).unwrap();
            r_upper.size_y = height_from_ceiling - (OBSTACLE_TIGTHFACTOR/2.0);
            r_upper.pos_x = WINDOW_W+50.0;
            
            let mut r_lower = rect.get_mut(e_lower).unwrap();
            r_lower.pos_y = height_from_ceiling+(OBSTACLE_TIGTHFACTOR/2.0);
            r_lower.size_y = WINDOW_H - (height_from_ceiling+(OBSTACLE_TIGTHFACTOR/2.0));
            r_lower.pos_x = WINDOW_W+50.0;

            if let (None, None) = (velocity.remove(e_upper),velocity.remove(e_lower))
            {
                println!("wtf happened here");
            }
        }

        let mut last_pos_x = 0.0;
        for (r, _, _) in (&rect, &obst, &velocity).join()
        {
            if r.pos_x > last_pos_x
            {
                last_pos_x = r.pos_x;
            }
        }
        
        // is the obstacle in front of us far enough for us to join?
        let mut spawned_ent = None;
        'spawn: for (ent_outer, r, obs, _) in (&entity, &rect, &obst, !&velocity).join()
        {
            if r.pos_x-last_pos_x > (WINDOW_W/OBST_AMOUNT as f32)
            {
                let id = obs.0;
                for (ent_inner, obs,r_inner,  _) in (&*entity, &obst, &rect, !&velocity).join()
                {
                    if id == obs.0 && r != r_inner
                    {
                        spawned_ent = Some(vec![
                            ent_outer,
                            ent_inner
                        ]);
                        break 'spawn;
                    }
                }
            }
        }

        if let Some(ents) = spawned_ent
        { //yes, add velocity
            for ent in ents
            {
                if let Err(err) = velocity.insert(ent,
                    Velocity { x: OBST_SPEED, y: 0.0 })
                {
                    println!("{err}");
                };
            }
        }
    }
}

struct CollisionSystem;
impl<'a> System<'a> for CollisionSystem
{
    type SystemData = (
        Write<'a, IsGameover>,
        ReadStorage<'a, Rect>,
        ReadStorage<'a, Collision>,
        ReadStorage<'a, Controllable>,
    );

    fn run(&mut self, data: Self::SystemData)
    {
        let (mut is_gameover, rect,collision, contr) 
            = data;

        // straight forward box collisions
        // https://developer.mozilla.org/en-US/docs/Games/Techniques/2D_collision_detection
        for (r_p, _, _) in (&rect, &collision, &contr).join()
        {
            for (r, _, _) in (&rect, &collision, !&contr).join()
            {
                if  r_p.pos_x < r.pos_x + r.size_x 
                &&  r_p.pos_x + r_p.size_x > r.pos_x
                &&  r_p.pos_y < r.pos_y + r.size_y 
                &&  r_p.pos_y + r_p.size_y > r.pos_y
                {
                    is_gameover.0 = true;
                }
            }
        }
    }
}

struct MovementSystem;
impl<'a> System<'a> for MovementSystem
{
    type SystemData = (
        WriteStorage<'a, Rect>,
        ReadStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData)
    {
        let (mut rect, velocity) 
            = data;

        for (velo, r) in (&velocity, &mut rect).join()
        {
            r.pos_y += velo.y;
            r.pos_x += velo.x;
        }
    }
}

struct ScoreSystem;
impl<'a> System<'a> for ScoreSystem
{
    type SystemData = (
        Entities<'a>,
        Write<'a, Score>,
        ReadStorage<'a, Rect>,
        WriteStorage<'a, Dirty>,
        ReadStorage<'a, Obstacle>,
        ReadStorage<'a, Controllable>,
    );

    fn run(&mut self, data: Self::SystemData)
    {
        let (entities, mut score, rect, mut dirty, obstacle, controllable) = data;

        // we don't want to add a score mutliple times for the same obstacle
        // so we use dirty component to keep track
        let mut is_dirty = None;
        'outer:for (r_player,_) in (&rect, &controllable).join()
        {
            for (r, obst, _) in (&rect, &obstacle, !&dirty).join()
            {
                if r_player.pos_x > r.pos_x
                {
                    is_dirty = Some(obst.0);
                    break 'outer;
                } 
            }
        }

        if let Some(obst_id) = is_dirty
        {
            for (ent, obst) in (&*entities, &obstacle).join()
            {
                if obst.0 == obst_id
                {
                    if let Err(err) = dirty.insert(ent,Dirty)
                    {
                        println!("{err}");
                    };
                }
            }
            score.0 += 1;
        }
    }
}

fn create_obstacles(world: &mut World, obstacle_amount: u8)
{
    let mut rng = rand::thread_rng();
    let mut is_first = true;
    
    //TODO make based on window _W instead of fixed amount or whatever
    for i in 0..obstacle_amount+1
    {
        let height_from_ceiling = (WINDOW_H/2.0) * rng.gen_range::<f32, f32, f32>(RNG_LOW, RNG_HIGH);
        if is_first //todo hack
        {
            world
                .create_entity()
                .with(Rect{ pos_x: WINDOW_W+50.0, pos_y: 0.0,
                    size_x: 2.0*SQUARE_SIZE/3.0, size_y: height_from_ceiling - (OBSTACLE_TIGTHFACTOR/2.0)})
                .with(Velocity { x: OBST_SPEED, y: 0.0 })
                .with(Collision)
                .with(Obstacle(i))
                .build();
            world
                .create_entity()
                .with(Rect{ pos_x: WINDOW_W+50.0, pos_y: height_from_ceiling+(OBSTACLE_TIGTHFACTOR/2.0),
                    size_x: 2.0*SQUARE_SIZE/3.0, size_y: WINDOW_H - (height_from_ceiling+(OBSTACLE_TIGTHFACTOR/2.0))})
                .with(Velocity { x: OBST_SPEED, y: 0.0 })
                .with(Collision)
                .with(Obstacle(i))
                .build();
        }
        else
        {
            world
                .create_entity()
                .with(Rect{ pos_x: WINDOW_W+50.0, pos_y: 0.0,
                    size_x: 2.0*SQUARE_SIZE/3.0, size_y: height_from_ceiling - (OBSTACLE_TIGTHFACTOR/2.0)})
                .with(Collision)
                .with(Obstacle(i))
                .build();
            world
                .create_entity()
                .with(Rect{ pos_x: WINDOW_W+50.0, pos_y: height_from_ceiling+(OBSTACLE_TIGTHFACTOR/2.0),
                    size_x: 2.0*SQUARE_SIZE/3.0, size_y: WINDOW_H - (height_from_ceiling+(OBSTACLE_TIGTHFACTOR/2.0))})
                .with(Collision)
                .with(Obstacle(i))
                .build();
        }
        is_first = false;
    }
}
