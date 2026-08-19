# bench/r6/Makefile — build the r6 front-end fuzz driver against the real
# Bebop front-end sources (lexer.c parser.c verify.c + deps), no CLI main.
NATIVE := /root/dowiz/bebop-lang/native
CC     ?= cc
CFLAGS := -O1 -std=c11 -Wall -Wextra -g -I$(NATIVE)/src

# Reuse the native SRC list, drop the CLI driver, prefix the native src dir.
NATIVE_SRC := $(shell sed -n 's/^SRC := //p' $(NATIVE)/Makefile)
SRC := $(filter-out src/main.c,$(NATIVE_SRC))
SRC := $(addprefix $(NATIVE)/,$(SRC))

r6_fuzz: r6_fuzz.c $(SRC)
	$(CC) $(CFLAGS) -o $@ r6_fuzz.c $(SRC) -lm -lpthread

clean:
	rm -f r6_fuzz crash_first.bin crash_first.txt

.PHONY: clean
