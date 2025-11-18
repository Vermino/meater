use anyhow::anyhow;
use uuid::uuid;

pub const SERVICE_UUID: uuid::Uuid = uuid!("a75cc7fc-c956-488f-ac2a-2dbc08b63a04");
pub const BATTERY_UUID: uuid::Uuid = uuid!("2adb4877-68d8-4884-bd3c-d83853bf27b8");
pub const TEMPERATURE_UUID: uuid::Uuid = uuid!("7edda774-045e-4bbf-909b-45d1991a2876");

/// State the MEATER device may be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Disconnected,
    Connecting,
    Connected,
}

/// A reading from the MEATER probe containing temperature and battery data.
#[derive(Debug, Clone)]
pub struct ProbeReading {
    pub tip_temperature: f32,
    pub ambient_temperature: f32,
    pub battery_percent: u16,
    pub timestamp: std::time::SystemTime,
}

/// An event emitted by the MEATER client.
#[derive(Debug, Clone)]
pub enum Event {
    /// State changed.
    State(State),
    /// Temperature changed.
    Temperature { tip: f32, ambient: f32 },
    /// Battery level changed.
    Battery { percent: u16 },
}

impl Event {
    /// Parse a temperature notification from raw bytes.
    pub fn parse_temperature(value: &[u8]) -> anyhow::Result<Self> {
        if value.len() != 8 {
            return Err(anyhow!(
                "temperature does not contain correct number of bytes"
            ));
        }

        let tip = to_u16(value[1], value[0]);
        let ra = to_u16(value[3], value[2]);
        let oa = to_u16(value[5], value[4]);
        let ambient = tip + ((ra - 48.min(oa)) * 16 * 589) / 1487;

        Ok(Event::Temperature {
            tip: to_degree_celsius(tip),
            ambient: to_degree_celsius(ambient),
        })
    }

    /// Parse a battery notification from raw bytes.
    pub fn parse_battery(value: &[u8]) -> anyhow::Result<Self> {
        if value.len() < 2 {
            return Err(anyhow!("battery does not contain correct number of bytes"));
        }

        Ok(Event::Battery {
            percent: to_u16(value[1], value[0]) * 10,
        })
    }
}

fn to_u16(msb: u8, lsb: u8) -> u16 {
    u16::from(msb) * 256 + u16::from(lsb)
}

fn to_degree_celsius(value: u16) -> f32 {
    (f32::from(value) + 8.0) / 16.0
}
