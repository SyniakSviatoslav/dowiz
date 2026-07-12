export function distanceKm(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const R = 6371; // Earth's radius in km
  const dLat = (lat2 - lat1) * Math.PI / 180;
  const dLon = (lon2 - lon1) * Math.PI / 180;
  
  const a = 
    Math.sin(dLat/2) * Math.sin(dLat/2) +
    Math.cos(lat1 * Math.PI / 180) * Math.cos(lat2 * Math.PI / 180) * 
    Math.sin(dLon/2) * Math.sin(dLon/2);
    
  const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1-a));
  const distance = R * c;
  
  return Math.round(distance * 1000) / 1000;
}

/**
 * Naive ETA calculation based on average urban speed (25 km/h).
 * Returns ETA in seconds.
 */
export function calculateNaiveETASeconds(distanceKm: number): number {
  const averageSpeedKmh = 25;
  return Math.round((distanceKm / averageSpeedKmh) * 3600);
}

/**
 * Validates if the given coordinate is within a specified radius of the center.
 */
export function isWithinGeofence(
  lat1: number, lon1: number,
  lat2: number, lon2: number,
  radiusKm: number
): boolean {
  return distanceKm(lat1, lon1, lat2, lon2) <= radiusKm;
}

/**
 * Rounds the coordinate to 5 decimal places for privacy and standardized precision.
 */
export function roundCoordinate(coord: number): number {
  return Math.round(coord * 100000) / 100000;
}
