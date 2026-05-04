import { useState, useEffect } from "react";

const DAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTH_NAMES = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

export function useClock() {
  const [now, setNow] = useState(new Date());

  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(id);
  }, []);

  return {
    hh: now.getHours().toString().padStart(2, "0"),
    mm: now.getMinutes().toString().padStart(2, "0"),
    dayName: DAY_NAMES[now.getDay()],
    day: now.getDate().toString().padStart(2, "0"),
    monthName: MONTH_NAMES[now.getMonth()],
    year: now.getFullYear(),
  };
}
