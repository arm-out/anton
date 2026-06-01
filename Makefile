ifndef EXE
    EXE := anton
endif

openbench:
	@echo Compiling $(EXE) for OpenBench
	cargo rustc --release --bin anton -- -C target-cpu=native --emit link=$(EXE)
