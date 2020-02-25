#!/bin/bash
set -e
REQUIRED_ENVS=("CMAKE_SOURCE_DIR" "CMAKE_BINARY_DIR"
"MESATEE_OUT_DIR" "MESATEE_TARGET_DIR" "RUSTUP_TOOLCHAIN" "MESAPY_VERSION"
"SGX_EDGER8R" "MT_EDL_FILE" "SGX_SDK" "RUST_SGX_SDK" "CMAKE_C_COMPILER"
"CMAKE_AR" "SGX_UNTRUSTED_CFLAGS" "SGX_TRUSTED_CFLAGS" "MT_SCRIPT_DIR"
"MESATEE_SERVICE_INSTALL_DIR" "MESATEE_EXAMPLE_INSTALL_DIR" "MESATEE_BIN_INSTALL_DIR"
"MESATEE_CLI_INSTALL_DIR" "MESATEE_DCAP_INSTALL_DIR" "MESATEE_LIB_INSTALL_DIR" "MESATEE_TEST_INSTALL_DIR"
"MESATEE_AUDITORS_DIR" "MESATEE_EXAMPLE_AUDITORS_DIR" "DCAP" "MESATEE_SYMLINKS"
)

for var in "${REQUIRED_ENVS[@]}"; do
    [ -z "${!var}" ] && echo "Please set ${var}" && exit -1
done

${MT_SCRIPT_DIR}/setup_cmake_tomls ${CMAKE_SOURCE_DIR} ${CMAKE_BINARY_DIR}
mkdir -p ${MESATEE_OUT_DIR} ${MESATEE_TARGET_DIR} ${MESATEE_SERVICE_INSTALL_DIR} \
      ${MESATEE_EXAMPLE_INSTALL_DIR} ${MESATEE_CLI_INSTALL_DIR} \
      ${MESATEE_BIN_INSTALL_DIR} ${MESATEE_LIB_INSTALL_DIR} \
    ${MESATEE_TEST_INSTALL_DIR} ${MESATEE_AUDITORS_DIR} ${MESATEE_EXAMPLE_AUDITORS_DIR}
if [ -n "$DCAP" ]; then
    mkdir -p ${MESATEE_DCAP_INSTALL_DIR}
    cp ${CMAKE_SOURCE_DIR}/dcap/Rocket.toml ${MESATEE_DCAP_INSTALL_DIR}/Rocket.toml
    cp ${CMAKE_SOURCE_DIR}/keys/dcap_server_cert.pem ${MESATEE_DCAP_INSTALL_DIR}/
    cp ${CMAKE_SOURCE_DIR}/keys/dcap_server_key.pem ${MESATEE_DCAP_INSTALL_DIR}/
fi
# copy auditors to install directory to make it easy to package all built things
cp -RT ${CMAKE_SOURCE_DIR}/keys/auditors/ ${MESATEE_AUDITORS_DIR}/
cp ${CMAKE_SOURCE_DIR}/config/runtime.config.toml ${MESATEE_SERVICE_INSTALL_DIR}
cp ${CMAKE_SOURCE_DIR}/config/runtime.config.toml ${MESATEE_TEST_INSTALL_DIR}
cp -r ${CMAKE_SOURCE_DIR}/tests/fixtures/ ${MESATEE_TEST_INSTALL_DIR}
ln -f -s ${MESATEE_TEST_INSTALL_DIR}/fixtures ${MESATEE_SERVICE_INSTALL_DIR}/fixtures
cp -r ${CMAKE_SOURCE_DIR}/tests/scripts/ ${MESATEE_TEST_INSTALL_DIR}
# create the following symlinks to make remapped paths accessible and avoid repeated building
mkdir -p ${MESATEE_SYMLINKS}
ln -snf ${HOME}/.cargo ${MESATEE_SYMLINKS}/cargo_home
ln -snf ${CMAKE_SOURCE_DIR} ${MESATEE_SYMLINKS}/mesatee_src
ln -snf ${CMAKE_BINARY_DIR} ${MESATEE_SYMLINKS}/mesatee_build
# cleanup sgx_unwind/libunwind
(cd ${CMAKE_SOURCE_DIR}/third_party/crates-sgx/ && git clean -fdx vendor/sgx_unwind/libunwind/)
if git submodule status | egrep -q '^[-]|^[+]'; then echo 'INFO: Need to reinitialize git submodules' && git submodule update --init --recursive; fi
rustup install --no-self-update ${RUSTUP_TOOLCHAIN} > /dev/null 2>&1
# get mesapy
if [ ! -f ${MESATEE_OUT_DIR}/libpypy-c.a ] || [ ! -f ${MESATEE_OUT_DIR}/${MESAPY_VERSION}-mesapy-sgx.tar.gz ]; then
    cd ${MESATEE_OUT_DIR};
    echo "Downloading MesaPy ${MESAPY_VERSION}..."
    wget -qN https://mesapy.org/release/${MESAPY_VERSION}-mesapy-sgx.tar.gz;
    tar xzf ${MESAPY_VERSION}-mesapy-sgx.tar.gz;
    cd -
fi
# build libEnclave_u.a & libEnclave_t.o
if [ ! -f ${MESATEE_OUT_DIR}/libEnclave_u.a ]; then
    echo 'INFO: Start to build EDL.'
    ${SGX_EDGER8R} --untrusted ${MT_EDL_FILE} --search-path ${SGX_SDK}/include \
        --search-path ${RUST_SGX_SDK}/edl --untrusted-dir ${MESATEE_OUT_DIR}
    cd ${MESATEE_OUT_DIR}
    ${CMAKE_C_COMPILER} ${SGX_UNTRUSTED_CFLAGS} -c Enclave_u.c -o libEnclave_u.o
    ${CMAKE_AR} rcsD libEnclave_u.a libEnclave_u.o
    ${SGX_EDGER8R} --trusted ${MT_EDL_FILE} --search-path ${SGX_SDK}/include \
        --search-path ${RUST_SGX_SDK}/edl --trusted-dir ${MESATEE_OUT_DIR}
    ${CMAKE_C_COMPILER} ${SGX_TRUSTED_CFLAGS} -c Enclave_t.c -o libEnclave_t.o
fi
