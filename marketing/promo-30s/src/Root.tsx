import { Composition } from "remotion";
import { DowizPromo, FPS, DURATION_S } from "./DowizPromo";

export const Root: React.FC = () => (
  <Composition
    id="DowizPromo"
    component={DowizPromo}
    durationInFrames={FPS * DURATION_S}
    fps={FPS}
    width={1080}
    height={1920}
    defaultProps={{ lang: "en" as const, music: null, musicInSeconds: 12 }}
  />
);
