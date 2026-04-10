// Repository traits and implementations live here.
// Each repository gets its own submodule following this layout:
//
//   repositories/<RepositoryName>/mod.rs      — trait definition
//   repositories/<RepositoryName>/models.rs   — data models
//   repositories/<RepositoryName>/mock.rs     — mockall mock for unit tests
//   repositories/<RepositoryName>/sqlite.rs   — SQLite implementation

pub mod workspace;
