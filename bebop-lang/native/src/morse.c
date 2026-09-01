/* Bebop Morse — implementation (ITU codebook). Zero dependencies. */
#include "morse.h"

#include <ctype.h>
#include <string.h>

typedef struct {
    char ch;
    const char *code;
} MorseEntry;

static const MorseEntry TABLE[] = {
    {'a', ".-"},   {'b', "-..."}, {'c', "-.-."}, {'d', "-.."},  {'e', "."},
    {'f', "..-."}, {'g', "--."},  {'h', "...."}, {'i', ".."},   {'j', ".---"},
    {'k', "-.-"},  {'l', ".-.."}, {'m', "--"},   {'n', "-."},   {'o', "---"},
    {'p', ".--."}, {'q', "--.-"}, {'r', ".-."},  {'s', "..."},  {'t', "-"},
    {'u', "..-"},  {'v', "...-"}, {'w', ".--"},  {'x', "-..-"}, {'y', "-.--"},
    {'z', "--.."},
    {'0', "-----"}, {'1', ".----"}, {'2', "..---"}, {'3', "...--"},
    {'4', "....-"}, {'5', "....."}, {'6', "-...."}, {'7', "--..."},
    {'8', "---.."}, {'9', "----."},
};
#define N_TABLE (sizeof TABLE / sizeof TABLE[0])

static const MorseEntry *find_char(char c) {
    c = (char)tolower((unsigned char)c);
    for (size_t i = 0; i < N_TABLE; i++) {
        if (TABLE[i].ch == c) {
            return &TABLE[i];
        }
    }
    return NULL;
}

static const MorseEntry *find_code(const char *code, size_t len) {
    for (size_t i = 0; i < N_TABLE; i++) {
        if (strlen(TABLE[i].code) == len && strncmp(TABLE[i].code, code, len) == 0) {
            return &TABLE[i];
        }
    }
    return NULL;
}

int bp_morse_encode(const char *text, char *out, size_t cap) {
    size_t pos = 0;
    for (const char *p = text; *p; p++) {
        if (*p == ' ') {
            /* word separator: replace the trailing letter separator with '/' */
            if (pos > 0 && out[pos - 1] == ' ') {
                pos--;
            }
            if (pos + 1 >= cap) {
                return -1;
            }
            out[pos++] = '/';
            out[pos] = '\0';
            continue;
        }
        const MorseEntry *e = find_char(*p);
        if (!e) {
            return -1;
        }
        size_t clen = strlen(e->code);
        if (pos + clen + 1 >= cap) {
            return -1;
        }
        memcpy(out + pos, e->code, clen);
        pos += clen;
        out[pos++] = ' ';
        out[pos] = '\0';
    }
    /* strip trailing letter separator */
    if (pos > 0 && out[pos - 1] == ' ') {
        pos--;
        out[pos] = '\0';
    }
    return 0;
}

int bp_morse_decode(const char *morse, char *out, size_t cap) {
    size_t pos = 0;
    const char *p = morse;
    while (*p) {
        if (*p == ' ') {
            p++;
            continue;
        }
        if (*p == '/') {
            if (pos + 1 >= cap) {
                return -1;
            }
            out[pos++] = ' ';
            p++;
            continue;
        }
        const char *start = p;
        while (*p == '.' || *p == '-') {
            p++;
        }
        size_t len = (size_t)(p - start);
        if (len == 0) {
            p++;
            continue;
        }
        const MorseEntry *e = find_code(start, len);
        if (!e) {
            return -1;
        }
        if (pos + 1 >= cap) {
            return -1;
        }
        out[pos++] = e->ch;
    }
    out[pos] = '\0';
    return 0;
}
