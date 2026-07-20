// RED fixture for local/no-arbitrary-tailwind — arbitrary bracket values bypass the design scale.
export function Bad() {
  return (
    <div className="p-[13px] w-[100px] text-[#ff0066] gap-[7px]">
      off-scale spacing and off-token colour
    </div>
  );
}
