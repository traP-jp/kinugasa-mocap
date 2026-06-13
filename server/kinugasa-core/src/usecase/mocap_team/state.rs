use crate::domain::model::{mocap_team, unit_of_work};

#[derive(Debug, Clone)]
pub struct MocapTeamApiState<R, U>
where
    R: mocap_team::MocapTeamRepository<UoW = U::UoW>,
    U: unit_of_work::UnitOfWorkProvider,
{
    repository: R,
    unit_of_work_provider: U,
}

impl<R, U> MocapTeamApiState<R, U>
where
    R: mocap_team::MocapTeamRepository<UoW = U::UoW>,
    U: unit_of_work::UnitOfWorkProvider,
{
    pub fn new(repository: R, unit_of_work_provider: U) -> Self {
        Self {
            repository,
            unit_of_work_provider,
        }
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub fn unit_of_work_provider(&self) -> &U {
        &self.unit_of_work_provider
    }
}
