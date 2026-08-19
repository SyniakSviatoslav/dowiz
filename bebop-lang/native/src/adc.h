#ifndef BEBOP_ADC_H
#define BEBOP_ADC_H
#include <stddef.h>
#include <stdint.h>

/* Voltage/current sensor interface (MMIO, I2C, or flight-controller UART).
 * Paste #22: bare-metal ADC read, I2C sensor (INA219/INA226), telemetry. */

double adc_read_voltage(void);  /* MMIO: read ADC register, convert to volts */
double adc_read_current(void);  /* MMIO: read current shunt register */

/* I2C sensor stubs (INA219 at addr 0x40). Returns raw register value. */
int i2c_read_reg(uint8_t addr, uint8_t reg, uint16_t *val);
int i2c_write_reg(uint8_t addr, uint8_t reg, uint16_t val);

/* Parse MSP/MAVLink telemetry frame from flight controller UART buffer.
 * Extracts battery voltage (millivolts). Returns 0 on success. */
int telemetry_parse_msp(const uint8_t *buf, size_t len, double *voltage_out);

int adc_self_test(char *out, size_t cap);
#endif
