// Copyright 2024 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use std::path::PathBuf;

use crate::{
    action::Action,
    components::{
        beta_programme::BetaProgramme, footer::Footer, help::HelpPopUp, home::Home,
        resource_allocation::ResourceAllocationInputBox, Component,
    },
    config::{AppData, Config},
    mode::{InputMode, Scene},
    tui,
};
use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use sn_peers_acquisition::PeersArgs;
use tokio::sync::mpsc;

pub struct App {
    pub config: Config,
    pub app_data: AppData,
    pub tick_rate: f64,
    pub frame_rate: f64,
    pub components: Vec<Box<dyn Component>>,
    pub should_quit: bool,
    pub should_suspend: bool,
    pub input_mode: InputMode,
    pub scene: Scene,
    pub last_tick_key_events: Vec<KeyEvent>,
}

impl App {
    pub fn new(
        tick_rate: f64,
        frame_rate: f64,
        peers_args: PeersArgs,
        safenode_path: Option<PathBuf>,
    ) -> Result<Self> {
        let app_data = AppData::load()?;

        let home = Home::new(
            app_data.allocated_disk_space,
            &app_data.discord_username,
            peers_args,
            safenode_path,
        )?;
        let config = Config::new()?;
        let discord_username_input = BetaProgramme::new(app_data.discord_username.clone());
        let resource_allocation_input =
            ResourceAllocationInputBox::new(app_data.allocated_disk_space)?;
        let footer = Footer::default();
        let help = HelpPopUp::default();

        Ok(Self {
            config,
            app_data,
            tick_rate,
            frame_rate,
            components: vec![
                Box::new(footer),
                Box::new(home),
                Box::new(discord_username_input),
                Box::new(resource_allocation_input),
                Box::new(help),
            ],
            should_quit: false,
            should_suspend: false,
            input_mode: InputMode::Navigation,
            scene: Scene::Home,
            last_tick_key_events: Vec::new(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();

        let mut tui = tui::Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        // tui.mouse(true);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(action_tx.clone())?;
            component.register_config_handler(self.config.clone())?;
            component.init(tui.size()?)?;
        }

        loop {
            if let Some(e) = tui.next().await {
                match e {
                    tui::Event::Quit => action_tx.send(Action::Quit)?,
                    tui::Event::Tick => action_tx.send(Action::Tick)?,
                    tui::Event::Render => action_tx.send(Action::Render)?,
                    tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
                    tui::Event::Key(key) => {
                        if self.input_mode == InputMode::Navigation {
                            if let Some(keymap) = self.config.keybindings.get(&self.scene) {
                                if let Some(action) = keymap.get(&vec![key]) {
                                    info!("Got action: {action:?}");
                                    action_tx.send(action.clone())?;
                                } else {
                                    // If the key was not handled as a single key action,
                                    // then consider it for multi-key combinations.
                                    self.last_tick_key_events.push(key);

                                    // Check for multi-key combinations
                                    if let Some(action) = keymap.get(&self.last_tick_key_events) {
                                        info!("Got action: {action:?}");
                                        action_tx.send(action.clone())?;
                                    }
                                }
                            };
                        } else if self.input_mode == InputMode::Entry {
                            for component in self.components.iter_mut() {
                                let send_back_actions = component.handle_events(Some(e.clone()))?;
                                for action in send_back_actions {
                                    action_tx.send(action)?;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            while let Ok(action) = action_rx.try_recv() {
                if action != Action::Tick && action != Action::Render {
                    debug!("{action:?}");
                }
                match action {
                    Action::Tick => {
                        self.last_tick_key_events.drain(..);
                    }
                    Action::Quit => self.should_quit = true,
                    Action::Suspend => self.should_suspend = true,
                    Action::Resume => self.should_suspend = false,
                    Action::Resize(w, h) => {
                        tui.resize(Rect::new(0, 0, w, h))?;
                        tui.draw(|f| {
                            for component in self.components.iter_mut() {
                                let r = component.draw(f, f.size());
                                if let Err(e) = r {
                                    action_tx
                                        .send(Action::Error(format!("Failed to draw: {:?}", e)))
                                        .unwrap();
                                }
                            }
                        })?;
                    }
                    Action::Render => {
                        tui.draw(|f| {
                            for component in self.components.iter_mut() {
                                let r = component.draw(f, f.size());
                                if let Err(e) = r {
                                    action_tx
                                        .send(Action::Error(format!("Failed to draw: {:?}", e)))
                                        .unwrap();
                                }
                            }
                        })?;
                    }
                    Action::SwitchScene(scene) => {
                        info!("Scene swtiched to: {scene:?}");
                        self.scene = scene;
                    }
                    Action::SwitchInputMode(mode) => {
                        info!("Input mode switched to: {mode:?}");
                        self.input_mode = mode;
                    }
                    Action::StoreDiscordUserName(ref username) => {
                        debug!("Storing discord username: {username:?}");
                        self.app_data.discord_username.clone_from(username);
                        self.app_data.save()?;
                    }
                    Action::StoreAllocatedDiskSpace(space) => {
                        debug!("Storing allocated disk space: {space:?}");
                        self.app_data.allocated_disk_space = space;
                        self.app_data.save()?;
                    }
                    _ => {}
                }
                for component in self.components.iter_mut() {
                    if let Some(action) = component.update(action.clone())? {
                        action_tx.send(action)?
                    };
                }
            }
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                tui = tui::Tui::new()?
                    .tick_rate(self.tick_rate)
                    .frame_rate(self.frame_rate);
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }
}
