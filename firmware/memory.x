MEMORY
{
  /* 0x08000000..0x08005000 is the vendor USB bootloader, the only recovery path without a probe. */
  /* Flash size register 0x1FFFF7E0 reads 256 (KB). */
  FLASH : ORIGIN = 0x08005000, LENGTH = 256K - 20K
  /* 64K is safe on every candidate chip; raise to 96K once the part is confirmed. */
  RAM : ORIGIN = 0x20000000, LENGTH = 64K
}

/* Code that must run without flash wait states: only the first 128 KB of flash has none, and
   the end of .text is past that. Placed right after the vector table, ahead of .text. */
SECTIONS
{
  .fast_text : ALIGN(4)
  {
    *(.fast_text .fast_text.*);
  } > FLASH
} INSERT AFTER .vector_table;
_stext = ALIGN(ADDR(.fast_text) + SIZEOF(.fast_text), 4);
