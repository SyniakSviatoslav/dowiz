/* Bebop Money — implementation (port of dowiz money.rs). */
#include "money.h"

#include <stdio.h>
#include <string.h>

const char *money_currency_code(Currency c) {
    switch (c) {
        case CUR_ALL: return "ALL";
        case CUR_EUR: return "EUR";
        case CUR_USD: return "USD";
    }
    return "?";
}

int money_currency_from_code(const char *s, Currency *out) {
    if (strcmp(s, "ALL") == 0) { *out = CUR_ALL; return 0; }
    if (strcmp(s, "EUR") == 0) { *out = CUR_EUR; return 0; }
    if (strcmp(s, "USD") == 0) { *out = CUR_USD; return 0; }
    return -1;
}

Money money_new(int64_t minor, Currency c) {
    Money m;
    m.minor = minor;
    m.currency = c;
    return m;
}

int money_checked_add(Money a, Money b, Money *out, char *err, size_t cap) {
    if (a.currency != b.currency) {
        snprintf(err, cap, "cross-currency add rejected: %s + %s",
                 money_currency_code(a.currency), money_currency_code(b.currency));
        return -1;
    }
    int64_t minor;
    if (__builtin_add_overflow(a.minor, b.minor, &minor)) {
        snprintf(err, cap, "money add overflow");
        return -1;
    }
    *out = money_new(minor, a.currency);
    return 0;
}

int money_checked_neg(Money a, Money *out, char *err, size_t cap) {
    if (a.minor == INT64_MIN) {
        snprintf(err, cap, "money neg overflow (i64::MIN has no additive inverse)");
        return -1;
    }
    *out = money_new(-a.minor, a.currency);
    return 0;
}

int money_checked_sub(Money a, Money b, Money *out, char *err, size_t cap) {
    if (a.currency != b.currency) {
        snprintf(err, cap, "cross-currency sub rejected: %s - %s",
                 money_currency_code(a.currency), money_currency_code(b.currency));
        return -1;
    }
    int64_t minor;
    if (__builtin_sub_overflow(a.minor, b.minor, &minor)) {
        snprintf(err, cap, "money sub overflow");
        return -1;
    }
    *out = money_new(minor, a.currency);
    return 0;
}

int money_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[128];
    Money r;
#define M(cond, name)                                                \
    do {                                                             \
        int r2 = snprintf(out + pos, cap - pos, "[%s] %s\n",         \
                          (cond) ? "ok" : "FAIL", name);             \
        if (r2 > 0) pos += (size_t)r2;                               \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    M(money_currency_from_code("EUR", &r.currency) == 0 &&
          r.currency == CUR_EUR,
      "currency from code");
    M(money_currency_from_code("BTC", &r.currency) == -1,
      "unknown currency rejected");

    Money a = money_new(100, CUR_EUR);
    Money b = money_new(200, CUR_EUR);
    M(money_checked_add(a, b, &r, err, sizeof err) == 0 && r.minor == 300,
      "checked_add 100+200 == 300");

    Money u = money_new(50, CUR_USD);
    M(money_checked_add(a, u, &r, err, sizeof err) == -1 &&
          strstr(err, "cross-currency") != NULL,
      "cross-currency add rejected");

    Money mx = money_new(INT64_MAX, CUR_EUR);
    Money one = money_new(1, CUR_EUR);
    M(money_checked_add(mx, one, &r, err, sizeof err) == -1 &&
          strstr(err, "overflow") != NULL,
      "add overflow rejected");

    Money five = money_new(5, CUR_EUR);
    M(money_checked_neg(five, &r, err, sizeof err) == 0 && r.minor == -5,
      "neg(5) == -5");

    Money mmin = money_new(INT64_MIN, CUR_EUR);
    M(money_checked_neg(mmin, &r, err, sizeof err) == -1,
      "neg(i64::MIN) rejected");

    M(money_checked_sub(a, b, &r, err, sizeof err) == 0 && r.minor == -100,
      "checked_sub 100-200 == -100");
    M(money_checked_sub(mmin, one, &r, err, sizeof err) == -1,
      "sub overflow rejected");

    return all_ok ? 0 : -1;
}
