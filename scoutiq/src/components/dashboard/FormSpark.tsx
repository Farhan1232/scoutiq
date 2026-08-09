interface FormSparkProps {
  values: number[];
}

export function FormSpark({ values }: FormSparkProps) {
  const max = Math.max(...values);
  const min = Math.min(...values);
  const range = Math.max(max - min, 0.4);

  const points = values
    .map((value, index) => {
      const x = (index / (values.length - 1)) * 56;
      const y = 18 - ((value - min) / range) * 14;
      return `${x},${y}`;
    })
    .join(" ");

  return (
    <svg
      className="form-spark"
      width="56"
      height="22"
      viewBox="0 0 56 22"
      aria-hidden="true"
    >
      <polyline
        points={points}
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
