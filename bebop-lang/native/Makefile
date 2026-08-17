# Bebop native compiler — Phase 1 (C bootstrap)
# Native, not Rust. Zero external dependencies (libc only). Self-hosts later.
CC      ?= cc
CFLAGS  ?= -O2 -std=c11 -Wall -Wextra -Wpedantic
BUILD   ?= build
BIN     := $(BUILD)/bebopc

SRC := src/main.c src/glyph.c src/lexer.c src/parser.c src/morse.c src/qtt.c src/ntt.c src/hyper.c src/mem.c

all: $(BIN)

$(BIN): $(SRC) src/glyph.h src/lexer.h src/parser.h src/morse.h src/qtt.h src/ntt.h src/hyper.h src/mem.h
	@mkdir -p $(BUILD)
	$(CC) $(CFLAGS) -o $@ $(SRC)

test: $(BIN)
	$(BIN) version
	$(BIN) glyphs
	$(BIN) glyph fn
	$(BIN) glyph struct
	@echo "--- tokenize sample ---"
	$(BIN) tokens ../samples/hello.bp
	@echo "--- parse sample ---"
	$(BIN) parse ../samples/hello.bp
	@echo "--- parse morse sample ---"
	$(BIN) parse ../samples/morse.bp
	@echo "--- fmt round-trip (idempotent) ---"
	$(BIN) fmt ../samples/hello.bp
	$(BIN) fmt ../samples/morse.bp
	@echo "--- morse ---"
	$(BIN) morse "ntt"
	$(BIN) unmorse "... --- ..."
	$(BIN) unmorse "$$($(BIN) morse 'quantum hypervector')"
	@echo "--- qtt semiring ---"
	$(BIN) qtt
	@echo "--- ntt ---"
	$(BIN) ntt
	@echo "--- hypervector ---"
	$(BIN) hyper
	@echo "--- memory ---"
	$(BIN) mem

clean:
	rm -rf $(BUILD)

.PHONY: all test clean
