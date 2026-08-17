/* Bebop Money — integer minor units, currency-tagged, overflow-safe (port of
 * dowiz money.rs). RED LINE: zero float arithmetic on money (C5); cross-currency
 * operations fail-closed (M5); all arithmetic is checked (no wrap/UB). */
#ifndef BEBOP_MONEY_H
#define BEBOP_MONEY_H

#include <stddef.h>
#include <stdint.h>

typedef enum { CUR_ALL, CUR_EUR, CUR_USD } Currency;

typedef struct {
    int64_t minor;
    Currency currency;
} Money;

const char *money_currency_code(Currency c);
int money_currency_from_code(const char *s, Currency *out);

Money money_new(int64_t minor, Currency c);

/* checked ops: return 0 on success (result in *out), -1 on error (err filled).
 * Error cases: cross-currency, or i64 overflow (never wrap). */
int money_checked_add(Money a, Money b, Money *out, char *err, size_t cap);
int money_checked_neg(Money a, Money *out, char *err, size_t cap);
int money_checked_sub(Money a, Money b, Money *out, char *err, size_t cap);

int money_self_test(char *out, size_t cap);

#endif /* BEBOP_MONEY_H */
