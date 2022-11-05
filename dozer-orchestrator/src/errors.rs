#![allow(clippy::enum_variant_names)]

use crossbeam::channel::RecvError;
use dozer_api::errors::GRPCError;
use dozer_core::dag::errors::ExecutionError;
use dozer_ingestion::errors::ConnectorError;
use dozer_types::errors::internal::BoxedError;
use dozer_types::thiserror;
use dozer_types::thiserror::Error;

#[derive(Error, Debug)]
pub enum OrchestrationError {
    #[error("Couldnt read file")]
    FailedToLoadFile(#[source] std::io::Error),
    #[error("Failed to parse dozer config..")]
    FailedToParseYaml(#[source] BoxedError),
    #[error("Failed to initialize dozer config..")]
    InitializationFailed,
    #[error("Failed to initialize api server..")]
    ApiServerFailed(#[source] std::io::Error),
    #[error("Failed to initialize grpc server..")]
    GrpcServerFailed(#[source] GRPCError),
    #[error("Failed to receive schema update for GRPC..")]
    SchemaUpdateFailed(#[source] RecvError),
    #[error("Ingestion message forwarding failed")]
    IngestionForwarderError,
    #[error(transparent)]
    ExecutionError(#[from] ExecutionError),
    #[error(transparent)]
    ConnectorError(#[from] ConnectorError),
}