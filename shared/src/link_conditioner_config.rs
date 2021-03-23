/// Contains configuration required to initialize a LinkConditioner
#[derive(Debug, Clone)]
pub struct LinkConditionerConfig {
    /// Delay to receive incoming messages in milliseconds
    pub incoming_latency: u32,
    /// The maximum additional random latency to delay received incoming
    /// messages in milliseconds. This may be added OR subtracted from the
    /// latency determined in the `incoming_latency` property above
    pub incoming_jitter: u32,
    /// The % chance that an incoming packet will be dropped.
    /// Represented as a value between 0 and 1
    pub incoming_loss: f32,
    /// The % chance that an incoming packet will have a single bit tampered
    /// with. Represented as a value between 0 and 1
    pub incoming_corruption: f32,
}

impl LinkConditionerConfig {
    /// Creates a new LinkConditionerConfig
    pub fn new(
        incoming_latency: u32,
        incoming_jitter: u32,
        incoming_loss: f32,
        incoming_corruption: f32,
    ) -> Self {
        LinkConditionerConfig {
            incoming_latency,
            incoming_jitter,
            incoming_loss,
            incoming_corruption,
        }
    }

    /// Creates a new LinkConditioner that simulates a connection which is in a
    /// good condition
    pub fn good_condition() -> Self {
        LinkConditionerConfig {
            incoming_latency: 50,
            incoming_jitter: 10,
            incoming_loss: 0.01,
            incoming_corruption: 0.0000015,
        }
    }

    /// Creates a new LinkConditioner that simulates a connection which is in an
    /// average condition
    pub fn average_condition() -> Self {
        LinkConditionerConfig {
            incoming_latency: 275,
            incoming_jitter: 20,
            incoming_loss: 0.055,
            incoming_corruption: 0.000015,
        }
    }

    /// Creates a new LinkConditioner that simulates a connection which is in an
    /// poor condition
    pub fn poor_condition() -> Self {
        LinkConditionerConfig {
            incoming_latency: 500,
            incoming_jitter: 30,
            incoming_loss: 0.1,
            incoming_corruption: 0.00015,
        }
    }
}
