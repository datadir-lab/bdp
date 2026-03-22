declare module 'regl-scatterplot' {
  export interface ScatterplotOptions {
    canvas: HTMLCanvasElement;
    pointSize?: number;
    opacity?: number;
    colorBy?: string;
    [key: string]: unknown;
  }

  export interface DrawOptions {
    x: number[];
    y: number[];
    [key: string]: unknown;
  }

  export interface SelectEventData {
    points: number[];
    [key: string]: unknown;
  }

  export interface Scatterplot {
    draw(options: DrawOptions): void;
    subscribe(event: string, callback: (data: unknown) => void): void;
    zoomToLocation(location: [number, number], speed: number, options?: { transition?: boolean }): void;
    destroy(): void;
    [key: string]: unknown;
  }

  export default function createScatterplot(options: ScatterplotOptions): Scatterplot;
}
