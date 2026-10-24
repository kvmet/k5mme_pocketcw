MEMORY
{
  /* 0x08000000..0x08005000 is the vendor USB bootloader, the only recovery path without a probe. */
  /* Flash size register 0x1FFFF7E0 reads 256 (KB). */
  FLASH : ORIGIN = 0x08005000, LENGTH = 256K - 20K
  /* 64K is safe on every candidate chip; raise to 96K once the part is confirmed. */
  RAM : ORIGIN = 0x20000000, LENGTH = 64K
}
