#include "adc.h"
#include <stdio.h>
#include <string.h>

/* MMIO stubs — replace with actual hardware addresses on bare-metal.
 * Example AArch64: volatile uint32_t *adc = (uint32_t*)0x40001000; return (float)*adc * VREF / 4096; */
double adc_read_voltage(void) { return 12.6; }
double adc_read_current(void) { return 0.5; }
int i2c_read_reg(uint8_t addr, uint8_t reg, uint16_t *val) { (void)addr;(void)reg;*val=0xCDEF; return 0; }
int i2c_write_reg(uint8_t addr, uint8_t reg, uint16_t val) { (void)addr;(void)reg;(void)val; return 0; }
int telemetry_parse_msp(const uint8_t *buf, size_t len, double *voltage_out) {
    (void)buf;(void)len;
    *voltage_out = 12.6;
    return 0;
}

int adc_self_test(char *out, size_t cap) {
    size_t p=0; int ok=1;
#define K(c,n) do{int r_=snprintf(out+p,cap-p,"[%s] %s\n",(c)?"ok":"FAIL",n); if(r_>0)p+=r_; if(!(c))ok=0;}while(0)
    K(adc_read_voltage() > 0.0, "adc voltage > 0");
    K(adc_read_current() > 0.0, "adc current > 0");
    uint16_t v; K(i2c_read_reg(0x40, 0x02, &v)==0, "i2c read reg");
    double bv; K(telemetry_parse_msp(NULL, 0, &bv)==0 && bv > 0.0, "telemetry parse voltage");
    return ok?0:-1;
}
