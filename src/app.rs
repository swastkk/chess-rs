use crate::board::{events::BoardEvent, Board};
use crate::data::{bitboard::BitBoard, BoardConfig};
use crate::generator::MoveGenerator;
use crate::ui::GuiFramework;

use log;
use pixels::{Error, Pixels, SurfaceTexture};
use std::cell::RefCell;
use std::rc::Rc;
use winit::dpi::LogicalSize;
use winit::event::Event;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const WIN_WIDTH: u32 = 1280;
const WIN_HEIGHT: u32 = 720;

pub struct App;

impl App {
    pub fn run() -> Result<(), Error> {
        let event_loop = EventLoop::new();
        let builder = WindowBuilder::new();
        let window_size = LogicalSize::new(WIN_WIDTH, WIN_HEIGHT);
        let window = builder
            .with_maximized(true)
            .with_title("chess-rs")
            .with_inner_size(window_size)
            .build(&event_loop)
            .unwrap();

        let mut board = Board::default();
        let config = Rc::new(RefCell::new(BoardConfig::default()));
        let generator = MoveGenerator::default();

        let (mut pixels, mut framework) = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            let board_size = board.get_draw_area_side();

            // TODO: use new_async for web
            let pixels = Pixels::new(board_size, board_size, surface_texture)?;
            let framework = GuiFramework::new(
                &event_loop,
                window_size.width,
                window_size.height,
                window.scale_factor() as f32,
                &pixels,
                config.clone(),
            );

            (pixels, framework)
        };

        let mut moves = BitBoard::from(0);
        event_loop.run(move |event, _, control_flow| {
            control_flow.set_poll();

            match event {
                Event::WindowEvent { event, .. } => {
                    // Update egui inputs
                    if !framework.handle_event(&event) {
                        use winit::event::WindowEvent::*;

                        match event {
                            CloseRequested => {
                                log::info!("The close Button was pressed.");
                                control_flow.set_exit();
                            }
                            Resized(size) => {
                                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                                    log::error!("Pixels failed to resize error: {}", err);
                                    control_flow.set_exit();
                                }
                                framework.resize(size.width, size.height);
                            }
                            ScaleFactorChanged {
                                scale_factor,
                                new_inner_size: _,
                            } => {
                                framework.scale_factor(scale_factor);
                            }
                            MouseInput { state, button, .. } => {
                                let board_event = BoardEvent::MouseInput { state, button };
                                board.handle_event(board_event, &(*config).borrow());
                            }
                            CursorMoved { position, .. } => {
                                if let Ok(pos) = pixels.window_pos_to_pixel(position.into()) {
                                    let board_event = BoardEvent::CursorMoved { position: pos };
                                    board.handle_event(board_event, &(*config).borrow());
                                } else {
                                    let board_event = BoardEvent::CursorLeft;
                                    board.handle_event(board_event, &(*config).borrow());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Event::MainEventsCleared => {
                    if let Some((prev, new)) = board.get_user_move() {
                        config.borrow_mut().move_xy_to_xy(prev, new);
                    }
                    if let Some((piece, pos)) = board.get_picked_piece() {
                        moves = generator.get_moves(piece, pos, &(*config).borrow());
                    } else {
                        moves = BitBoard::from(0);
                    }
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    // Redraw here
                    board.draw(pixels.get_frame_mut(), &(*config).borrow(), &moves);
                    // Prepare egui
                    framework.prepare(&window);
                    // Render everything together
                    let render_result = pixels.render_with(|encoder, render_target, context| {
                        // Render the board texture
                        context.scaling_renderer.render(encoder, render_target);
                        // Render egui
                        framework.render(encoder, render_target, context);
                        Ok(())
                    });

                    if let Err(err) = render_result {
                        log::error!("pixels.render_with failed: {err}");
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }
                _ => (),
            }
        });
    }
}