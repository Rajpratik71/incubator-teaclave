// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

#![cfg_attr(feature = "mesalock_sgx", no_std)]
#[cfg(feature = "mesalock_sgx")]
#[macro_use]
extern crate sgx_tstd as std;

#[macro_use]
extern crate log;

use anyhow::Result;
use std::prelude::v1::*;
use std::sync::Arc;
use std::thread;
use teaclave_attestation::RemoteAttestation;
use teaclave_ipc::proto::{
    ECallCommand, FinalizeEnclaveInput, FinalizeEnclaveOutput, InitEnclaveInput, InitEnclaveOutput,
    StartServiceInput, StartServiceOutput,
};
use teaclave_ipc::{handle_ecall, register_ecall_handler};
use teaclave_proto::teaclave_authentication_service::{
    TeaclaveAuthenticationRequest, TeaclaveAuthenticationResponse,
};
use teaclave_rpc::config::SgxTrustedTlsServerConfig;
use teaclave_rpc::server::SgxTrustedTlsServer;
use teaclave_service_enclave_utils::ServiceEnclave;

mod service;
mod user_db;
mod user_info;

fn start_endpoint(
    listener: std::net::TcpListener,
    db_client: user_db::DbClient,
    attestation: Arc<RemoteAttestation>,
) {
    let config = SgxTrustedTlsServerConfig::new_without_verifier(
        &attestation.cert,
        &attestation.private_key,
    )
    .unwrap();

    let mut server = SgxTrustedTlsServer::<
        TeaclaveAuthenticationResponse,
        TeaclaveAuthenticationRequest,
    >::new(listener, &config);

    let service = service::TeaclaveAuthenticationService::new(db_client).unwrap();
    match server.start(service) {
        Ok(_) => (),
        Err(e) => {
            error!("Service exit, error: {}.", e);
        }
    }
}

#[handle_ecall]
fn handle_start_service(args: &StartServiceInput) -> Result<StartServiceOutput> {
    debug!("handle_start_service");
    let api_listener = std::net::TcpListener::new(args.fds[0])?;
    let internal_listener = std::net::TcpListener::new(args.fds[1])?;
    let ias_config = &args.config.ias.as_ref().unwrap();
    let attestation = Arc::new(
        RemoteAttestation::generate_and_endorse(&ias_config.ias_key, &ias_config.ias_spid).unwrap(),
    );
    let database = user_db::Database::open()?;

    let attestation_ref = attestation.clone();
    let client = database.get_client();
    let api_endpoint_thread_handler = thread::spawn(move || {
        start_endpoint(api_listener, client, attestation_ref);
    });

    let client = database.get_client();
    let internal_endpoint_thread_handler = thread::spawn(move || {
        start_endpoint(internal_listener, client, attestation);
    });

    api_endpoint_thread_handler.join().unwrap();
    internal_endpoint_thread_handler.join().unwrap();

    Ok(StartServiceOutput::default())
}

#[handle_ecall]
fn handle_init_enclave(_args: &InitEnclaveInput) -> Result<InitEnclaveOutput> {
    ServiceEnclave::init(env!("CARGO_PKG_NAME"))?;
    Ok(InitEnclaveOutput::default())
}

#[handle_ecall]
fn handle_finalize_enclave(_args: &FinalizeEnclaveInput) -> Result<FinalizeEnclaveOutput> {
    ServiceEnclave::finalize()?;
    Ok(FinalizeEnclaveOutput::default())
}

register_ecall_handler!(
    type ECallCommand,
    (ECallCommand::StartService, StartServiceInput, StartServiceOutput),
    (ECallCommand::InitEnclave, InitEnclaveInput, InitEnclaveOutput),
    (ECallCommand::FinalizeEnclave, FinalizeEnclaveInput, FinalizeEnclaveOutput),
);

#[cfg(feature = "enclave_unit_test")]
pub mod tests {
    use super::*;
    use teaclave_test_utils::*;

    pub fn run_tests() -> bool {
        run_tests!(
            service::tests::test_user_login,
            service::tests::test_user_authenticate,
            service::tests::test_user_register,
        )
    }
}
