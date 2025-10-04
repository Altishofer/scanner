// Custom type definitions to extend existing interfaces

interface ImageCapture {
  /**
   * The grabFrame() method of the ImageCapture interface takes a snapshot of the live video stream
   * @returns A Promise that resolves with an ImageBitmap object
   */
  grabFrame(): Promise<ImageBitmap>;
}

interface MediaTrackCapabilities {
  /**
   * The torch property indicates whether the user agent supports torch (flashlight) control
   */
  torch?: boolean;
}

interface MediaTrackConstraintSet {
  /**
   * The torch constraint controls the torch (flashlight) setting
   */
  torch?: boolean | ConstrainBoolean;
}
