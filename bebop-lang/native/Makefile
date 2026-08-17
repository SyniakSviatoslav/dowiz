# Bebop native compiler — Phase 1 (C bootstrap)
# Native, not Rust. Zero external dependencies (libc only). Self-hosts later.
CC      ?= cc
CFLAGS  ?= -O2 -std=c11 -Wall -Wextra -Wpedantic
BUILD   ?= build
BIN     := $(BUILD)/bebopc

SRC := src/main.c src/glyph.c src/lexer.c src/parser.c src/morse.c src/qtt.c src/ntt.c src/hyper.c src/mem.c src/expr.c src/verify.c src/vsa.c

all: $(BIN)

$(BIN): $(SRC) src/glyph.h src/lexer.h src/parser.h src/morse.h src/qtt.h src/ntt.h src/hyper.h src/mem.h src/expr.h src/verify.h src/vsa.h
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
	@echo "--- fmt round-trip (idempotent) ---"
	$(BIN) fmt ../samples/hello.bp
	@echo "--- morse codec (utility) ---"
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
	@echo "--- expr (parse→typecheck→eval) ---"
	$(BIN) expr "1 + 2 * 3"
	$(BIN) expr "let y = 2 in y * y"
	$(BIN) expr "if (1 == 1) then 10 else 20"
	@echo "--- verify (bounded contracts) ---"
	$(BIN) verify
	@echo "--- vsa identifiers ---"
	$(BIN) vsa

clean:
	rm -rf $(BUILD)

.PHONY: all test clean
