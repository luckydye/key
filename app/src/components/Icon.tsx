import { twMerge } from "tailwind-merge";

export function Icon(props: { name: keyof typeof ICONS; class?: string | string[] }) {
  return (
    <span
      aria-hidden="true"
      class={twMerge(
        "inline-block align-sub leading-none [&>svg]:block [&>svg]:h-[1em] [&>svg]:w-[1em]",
        props.class,
      )}
    >
      {ICONS[props.name || "unknown"]}
    </span>
  );
}

const ICONS = {
  unknown: (
    <svg viewBox="0 0 34 34" fill="none" xmlns="http://www.w3.org/2000/svg">
      <title>unknown</title>
      <path d="M2 32L32 2M2 2L32 32M1 1H33V33H1V1Z" stroke="currentColor" />
    </svg>
  ),
  atrium: (
    <svg viewBox="0 0 256 239" fill="none" xmlns="http://www.w3.org/2000/svg">
      <title>atrium</title>
      <path
        d="M204.466 235.481L140.037 5.34294C139.328 2.80938 141.227 0.296233 143.863 0.279262L187.233 8.38991e-05C189.036 -0.011519 190.623 1.18143 191.11 2.91316L255.849 233.33C256.564 235.875 254.647 238.398 251.998 238.398H208.318C206.524 238.398 204.949 237.205 204.466 235.481Z"
        fill="currentColor"
      />
      <path
        d="M0.564265 232.707L108.845 50.2782C110.642 47.2513 115.2 47.8566 116.14 51.2471L133.843 115.06C134.133 116.105 133.984 117.222 133.431 118.155L106.708 163.248H144.241C146.044 163.248 147.624 164.451 148.099 166.186L157.576 200.768C158.272 203.307 156.357 205.811 153.719 205.811H82.6663C82.2822 205.811 81.9161 205.759 81.5725 205.663L63.1378 236.771C62.4181 237.985 61.1093 238.73 59.6953 238.73H4.00535C0.907012 238.73 -1.01453 235.367 0.564265 232.707Z"
        fill="currentColor"
      />
    </svg>
  ),
  collapse: (
    <svg viewBox="0 0 165 165" fill="none" xmlns="http://www.w3.org/2000/svg">
      <title>collapse</title>
      <path
        d="M150 116L82 48L14 116"
        stroke="currentColor"
        stroke-width="12"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  ),
  expand: (
    <svg viewBox="0 0 165 165" fill="none" xmlns="http://www.w3.org/2000/svg">
      <title>expand</title>
      <path
        d="M15 49L83 117L151 49"
        stroke="currentColor"
        stroke-width="12"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  ),
  check: (
    <svg viewBox="0 0 17.121 13.141" fill="none" xmlns="http://www.w3.org/2000/svg">
      <title>check</title>
      <path
        d="M683,754.437l5.041,5.041L698,749.518"
        transform="translate(-681.939 -748.457)"
        stroke="currentColor"
        fill="none"
        stroke-width="3"
      />
    </svg>
  ),
  close: (
    <svg viewBox="0 0 165 165" fill="none" xmlns="http://www.w3.org/2000/svg">
      <title>close</title>
      <path
        fill-rule="evenodd"
        clip-rule="evenodd"
        d="M145.757 153.243C148.101 155.586 151.899 155.586 154.243 153.243C156.586 150.899 156.586 147.101 154.243 144.757L91.4853 82L154.243 19.2426C156.586 16.8995 156.586 13.1005 154.243 10.7574C151.899 8.4142 148.101 8.4142 145.757 10.7574L82 74.5147L18.2426 10.7573C15.8995 8.4142 12.1005 8.4142 9.75735 10.7573C7.41421 13.1005 7.41421 16.8995 9.75735 19.2426L72.5147 82L9.75736 144.757C7.41422 147.1 7.41422 150.899 9.75736 153.243C12.1005 155.586 15.8995 155.586 18.2426 153.243L82 89.4853L145.757 153.243Z"
        fill="white"
      />
    </svg>
  ),
  "arrow-left": (
    <svg viewBox="0 0 9 17" fill="none" xmlns="http://www.w3.org/2000/svg">
      <title>arrow-left</title>
      <path
        fill-rule="evenodd"
        clip-rule="evenodd"
        d="M8.53558 0.553632C8.7912 0.806945 8.7912 1.21765 8.53558 1.47096L1.58023 8.36365L8.53558 15.2563C8.7912 15.5096 8.7912 15.9203 8.53558 16.1737C8.27997 16.427 7.86553 16.427 7.60992 16.1737L0.191734 8.82231C-0.0638818 8.569 -0.0638817 8.1583 0.191734 7.90498L7.60992 0.553632C7.86553 0.300319 8.27997 0.300319 8.53558 0.553632Z"
        fill="white"
      />
    </svg>
  ),
  "arrow-right": (
    <svg viewBox="0 0 9 16" fill="none" xmlns="http://www.w3.org/2000/svg">
      <title>arrow-right</title>
      <path
        fill-rule="evenodd"
        clip-rule="evenodd"
        d="M0.191712 15.81C-0.063904 15.5567 -0.063904 15.146 0.191712 14.8927L7.14706 8L0.191712 1.10731C-0.0639038 0.853999 -0.0639038 0.443298 0.191712 0.189984C0.447328 -0.063329 0.861763 -0.063329 1.11738 0.189984L8.53556 7.54134C8.79118 7.79465 8.79118 8.20535 8.53556 8.45866L1.11738 15.81C0.861763 16.0633 0.447328 16.0633 0.191712 15.81Z"
        fill="white"
      />
    </svg>
  ),
};
