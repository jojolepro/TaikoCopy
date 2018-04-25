extern crate amethyst;

use amethyst::audio::output::Output;
use amethyst::renderer::Camera;
use amethyst::ecs::prelude::*;
use amethyst::input::InputHandler;
use amethyst::core::transform::Transform;
use amethyst::core::timing::{Stopwatch, Time};
use amethyst::input::InputEvent;
use amethyst::winit::VirtualKeyCode;
use amethyst::shrev::{EventChannel, ReaderId};
use amethyst::assets::AssetStorage;
use amethyst::audio::Source;

use resources::*;
use components::*;
use utils::*;

pub struct GameSystem {
    pub reader_id: Option<ReaderId<InputEvent<String>>>,
    pub start_time: f64,
}

impl<'a> System<'a> for GameSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, HitObject>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, Camera>,
        Read<'a, AssetStorage<Source>>,
        Write<'a, Time>,
        Read<'a, InputHandler<String, String>>,
        ReadExpect<'a, Sounds>,
        Read<'a, Option<Output>>,
        Read<'a, BeatMap>,
        Read<'a, Stopwatch>,
        Write<'a, EventChannel<InputEvent<String>>>,
        Write<'a, HitObjectQueue>,
        Write<'a, HitOffsets>,
        Write<'a, UserSettings>,
    );
    fn run(
        &mut self,
        (
            entities,
            mut hitobjects,
            mut transforms,
            cam,
            audio,
            mut time,
            input,
            sounds,
            audio_output,
            beatmap,
            stopwatch,
            mut events,
            mut hitqueue,
            mut hitoffsets,
            mut user_settings,
        ): Self::SystemData,
    ) {
        if self.reader_id.is_none() {
            self.reader_id = Some(events.register_reader());
        }

        if (self.start_time <= 0.0) {
            self.start_time = time.absolute_time_seconds();
        }

        let cur_time = time.absolute_time_seconds() - self.start_time;

        let cur_time = cur_time + user_settings.offset;

        let (mut r1, mut r2, mut b1, mut b2, mut offset_up, mut offset_down) =
            (false, false, false, false, false, false);
        for ev in events.read(self.reader_id.as_mut().unwrap()) {
            match ev {
                &InputEvent::KeyPressed { key_code, .. } => match key_code {
                    VirtualKeyCode::Z => r1 = true,
                    VirtualKeyCode::X => r2 = true,
                    VirtualKeyCode::N => b1 = true,
                    VirtualKeyCode::M => b2 = true,
                    VirtualKeyCode::Equals => offset_up = true,
                    VirtualKeyCode::Subtract => offset_down = true,
                    _ => {}
                },
                &InputEvent::KeyReleased { .. } => {}
                _ => {}
            }
        }

        if offset_up {
            user_settings.offset = user_settings.offset + 0.005;
            println!("Offset: {} ms", user_settings.offset * 1000.0);
        } else if offset_down {
            user_settings.offset = user_settings.offset - 0.005;
            println!("Offset: {} ms", user_settings.offset * 1000.0);
        }

        let mut dropped_offsets = Vec::new();
        while let Some(head) = (&mut hitqueue.queue).pop_front() {
            if head.time + beatmap.maxhitoffset < cur_time {
                hitoffsets.offsets.push(None);
                dropped_offsets.push(head.time);
            } else {
                hitqueue.queue.push_front(head);
                break;
            }
        }

        if r1 || r2 || b1 || b2 {
            let (red, dual) = get_key_press_type(r1, r2, b1, b2);

            if let Some(ref output) = *audio_output {
                if red {
                    output.play_once(
                        audio
                            .get(&sounds.normal)
                            .expect("Failed to find normal hitsound"),
                        0.03,
                    );
                } else {
                    output.play_once(
                        audio
                            .get(&sounds.clap)
                            .expect("Failed to find clap hitsound"),
                        0.03,
                    );
                }
                if dual {
                    output.play_once(
                        audio
                            .get(&sounds.finish)
                            .expect("Failed to find finish hitsound"),
                        0.03,
                    );
                }
            }

            //Get clickable object
            if let Some(head) = (&mut hitqueue.queue).pop_front() {
                if let (Some(offset), clicked) = check_hit(&beatmap, &head, cur_time, red, dual) {
                    if clicked {
                        hitoffsets.offsets.push(Some(offset));
                    } else {
                        hitoffsets.offsets.push(None);
                    }
                    dropped_offsets.push(head.time);
                } else {
                    //Put back into list if pressed but no hitobject was found
                    hitqueue.queue.push_front(head);
                }
            }
        }

        //println!("cur_time: {}", cur_time);
        'outer: for (entity, obj, tr) in (&*entities, &mut hitobjects, &mut transforms).join() {
            //Drop objects that weren't clicked fast enough
            for dropped_offset in dropped_offsets.iter() {
                if *dropped_offset == obj.time {
                    //Drop visual object
                    //println!("Dropped entity");
                    entities.delete(entity);
                    //continue 'outer;
                }
            }
            //Update object position
            tr.translation[0] = (((obj.time - cur_time) * 0.50) + 0.3) as f32; //TEMPORARY. TO TEST HIT JUDGEMENT
        }
    }
}