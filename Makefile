ifndef EXE
    EXE := anton
endif

openbench:
    @echo Compiling $(EXE) for OpenBench
    cargo build --release --bin anton -- -C target-cpu=native --emit link=$(EXE)