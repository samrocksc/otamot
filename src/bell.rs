//! Bell sound module for Pomodoro timer notifications.
//!
//! Uses rodio to generate a pleasant bell-like sound when a timer cycle ends.

use rodio::{source::SineWave, OutputStream, Sink, Source};
use std::time::Duration;

/// Bell sound configuration
#[derive(Debug, Clone)]
pub struct BellConfig {
    /// Whether the bell is enabled
    pub enabled: bool,
    /// Volume level (0.0 to 1.0)
    pub volume: f32,
    /// Duration of the bell in milliseconds
    pub duration_ms: u64,
    /// Frequency of the bell in Hz (default: 880 = A5 note)
    pub frequency: f32,
    /// The tune to play
    pub tune: crate::config::BellTune,
}

impl Default for BellConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.3,
            duration_ms: 500,
            frequency: 880.0, // A5 note - pleasant bell-like tone
            tune: crate::config::BellTune::Default,
        }
    }
}

/// Bell sound player wrapper
pub struct Bell {
    config: BellConfig,
    /// Output stream handle - kept alive for sound playback
    _stream: Option<(OutputStream, rodio::OutputStreamHandle)>,
}

impl Default for Bell {
    fn default() -> Self {
        Self::new(BellConfig::default())
    }
}

impl Bell {
    /// Create a new bell with the given configuration
    pub fn new(config: BellConfig) -> Self {
        let stream = OutputStream::try_default().ok();
        Self {
            config,
            _stream: stream,
        }
    }

    /// Update the bell configuration
    pub fn set_config(&mut self, config: BellConfig) {
        self.config = config;
    }

    /// Get the current configuration
    pub fn config(&self) -> &BellConfig {
        &self.config
    }

    /// Play the bell sound if enabled
    pub fn play(&self) {
        if !self.config.enabled {
            return;
        }

        // Try to get a fresh output stream for each playback
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                match self.config.tune {
                    crate::config::BellTune::Default => {
                        let source = SineWave::new(self.config.frequency)
                            .amplify(self.config.volume)
                            .take_duration(Duration::from_millis(self.config.duration_ms))
                            .fade_out(Duration::from_millis(200));
                        sink.append(source);
                    }
                    crate::config::BellTune::LaCukaracha => {
                        // Very simplified "La Cucaracha" with SineWaves
                        // Frequencies for C4, F4, A4...
                        let tune = [
                            (261.63, 200),
                            (261.63, 200),
                            (261.63, 200), // C C C
                            (349.23, 600), // F
                            (440.00, 600), // A
                            (261.63, 200),
                            (261.63, 200),
                            (261.63, 200), // C C C
                            (349.23, 600), // F
                            (440.00, 600), // A
                            (349.23, 300),
                            (349.23, 300),
                            (329.63, 300),
                            (329.63, 300),
                            (293.66, 300),
                            (293.66, 300),
                            (261.63, 600), // C
                        ];
                        for (freq, ms) in tune {
                            let source = SineWave::new(freq)
                                .amplify(self.config.volume)
                                .take_duration(Duration::from_millis(ms))
                                .fade_out(Duration::from_millis(50));
                            sink.append(source);
                        }
                    }
                    crate::config::BellTune::IceCreamTruck => {
                        // High pitched simple arpeggio
                        let tune = [
                            (523.25, 150),
                            (659.25, 150),
                            (783.99, 150),
                            (1046.50, 300),
                            (783.99, 150),
                            (1046.50, 450),
                        ];
                        for (freq, ms) in tune {
                            let source = SineWave::new(freq)
                                .amplify(self.config.volume)
                                .take_duration(Duration::from_millis(ms))
                                .fade_out(Duration::from_millis(50));
                            sink.append(source);
                        }
                    }
                }

                sink.detach();
                std::mem::forget(_stream);
            }
        }
    }

    /// Check if audio is available on this system
    pub fn is_available(&self) -> bool {
        self._stream.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bell_config_default() {
        let config = BellConfig::default();
        assert!(config.enabled);
        assert!(config.volume > 0.0 && config.volume <= 1.0);
        assert!(config.duration_ms > 0);
        assert!(config.frequency > 0.0);
    }

    #[test]
    fn test_bell_default() {
        let bell = Bell::default();
        assert!(bell.config().enabled);
    }

    #[test]
    fn test_bell_config_update() {
        let mut bell = Bell::default();
        let new_config = BellConfig {
            enabled: false,
            volume: 0.5,
            duration_ms: 1000,
            frequency: 440.0,
            tune: crate::config::BellTune::Default,
        };
        bell.set_config(new_config.clone());
        assert_eq!(bell.config().enabled, false);
        assert_eq!(bell.config().volume, 0.5);
        assert_eq!(bell.config().duration_ms, 1000);
        assert_eq!(bell.config().frequency, 440.0);
    }

    #[test]
    fn test_bell_disabled_does_not_play() {
        let config = BellConfig {
            enabled: false,
            ..Default::default()
        };
        let bell = Bell::new(config);
        // Should not panic or error when disabled
        bell.play();
    }
}
