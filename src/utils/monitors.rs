use std::collections::HashMap;
use display_info::DisplayInfo;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct XMonitor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    primary: bool,
    pub dev_id: String,
}

unsafe impl Send for XMonitor {}

pub struct Monitors {
    main: u32,
    monitors: HashMap<u32, XMonitor>,
}

impl Monitors {
    pub fn new() -> Self {
        let (monitors, main) = Self::setup_monitors();
        Monitors {
            main,
            monitors,
        }
    }

    pub fn change_monitor(&mut self, id: u32) -> bool {
        if !self.has_monitor(id) {
            return false;
        }
        self.main = id;
        true
    }

    pub fn get_monitor(&self) -> Option<&XMonitor> {
        self.monitors.get(&self.main)
    }

    pub fn get_monitor_id(&self) -> u32 {
        self.main
    }
    pub fn has_monitor(&self, id: u32) -> bool {
        self.monitors.contains_key(&id)
    }

    pub fn get_monitors(&self) -> Vec<u32> {
        let mut monitors = Vec::new();

        for x in self.monitors.iter() {
            monitors.push(x.0.clone());
        }

        monitors
    }

    fn setup_monitors() -> (HashMap<u32, XMonitor>, u32) {
        let mut monitors = HashMap::new();
        let mut main = 0;

        if let Ok(vec_display) = DisplayInfo::all() {
            for display in vec_display {
                monitors.insert(display.id, XMonitor {
                    x: display.x,
                    y: display.y,
                    height: display.height,
                    width: display.width,
                    primary: display.is_primary,
                    #[cfg(target_os = "windows")]
                    dev_id: display.raw_handle.0.to_string(),
                    #[cfg(target_os = "macos")]
                    dev_id: display.raw_handle.id.to_string(),
                    #[cfg(target_os = "linux")]
                    dev_id: format!(":{}", &{
                        let input = display.name;
                        let re = regex::Regex::new(r"\d+$").unwrap(); // Match one or more digits
                        if let Some(m) = re.find(&input) {
                            m.as_str().parse().unwrap()
                        } else {
                            display.id.to_string()
                        }
                    }),
                });

                if display.is_primary {
                    main = display.id;
                }
            }
        }

        (monitors, main)
    }
}