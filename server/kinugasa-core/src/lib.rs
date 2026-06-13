pub mod domain;
pub mod usecase;

pub mod id {
    pub use crate::domain::model::id::*;
}

pub mod mocap_studio {
    pub mod project {
        pub use crate::domain::service::mocap_studio::project::*;
    }

    pub mod types {
        pub mod event {
            pub use crate::domain::model::mocap_studio::event::*;
        }

        pub mod log {
            pub use crate::domain::model::mocap_studio::log::*;
            pub use crate::usecase::mocap_studio::MocapStudioLogRepository;
        }

        pub mod state {
            pub use crate::domain::model::mocap_studio::state::*;
        }
    }

    pub mod upcast {
        pub use crate::domain::service::mocap_studio::upcast::*;
    }
}

pub mod mocap_team {
    pub use crate::usecase::mocap_team::handlers;

    pub mod types {
        pub use crate::domain::model::mocap_team::*;
        pub use crate::usecase::mocap_team::MocapTeamRepository;
    }
}

pub mod unit_of_work {
    pub use crate::usecase::unit_of_work::*;
}
