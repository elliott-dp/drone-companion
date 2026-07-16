#pragma once
// MESSAGE CC_TELEMETRY_GPS PACKING

#define MAVLINK_MSG_ID_CC_TELEMETRY_GPS 54003


typedef struct __mavlink_cc_telemetry_gps_t {
 uint64_t fc_timestamp_us; /*< [us] FC monotonic time since PX4 boot.*/
 uint32_t sequence; /*<  Per-stream monotonic counter.*/
 int32_t lat; /*< [degE7] Latitude, WGS-84, degrees * 1e7.*/
 int32_t lon; /*< [degE7] Longitude, WGS-84, degrees * 1e7.*/
 int32_t alt; /*< [mm] Altitude AMSL, millimeters.*/
 float eph; /*< [m] Horizontal position uncertainty.*/
 float epv; /*< [m] Vertical position uncertainty.*/
 float ground_speed; /*< [m/s] Ground speed.*/
 float heading; /*< [rad] Course over ground, [-pi, pi] (NaN if unknown).*/
 uint16_t noise_per_ms; /*<  GPS noise indicator (receiver-specific; 0 if unavailable).*/
 uint16_t jamming_indicator; /*<  GPS jamming indicator (receiver-specific; 0 if unavailable).*/
 uint8_t fix_type; /*<  GNSS fix type (as PX4 sensor_gps fix_type).*/
 uint8_t satellites_used; /*<  Satellites used in solution.*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_telemetry_gps_t;

#define MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN 47
#define MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN 47
#define MAVLINK_MSG_ID_54003_LEN 47
#define MAVLINK_MSG_ID_54003_MIN_LEN 47

#define MAVLINK_MSG_ID_CC_TELEMETRY_GPS_CRC 189
#define MAVLINK_MSG_ID_54003_CRC 189



#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_GPS { \
    54003, \
    "CC_TELEMETRY_GPS", \
    14, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_gps_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_gps_t, sequence) }, \
         { "lat", NULL, MAVLINK_TYPE_INT32_T, 0, 12, offsetof(mavlink_cc_telemetry_gps_t, lat) }, \
         { "lon", NULL, MAVLINK_TYPE_INT32_T, 0, 16, offsetof(mavlink_cc_telemetry_gps_t, lon) }, \
         { "alt", NULL, MAVLINK_TYPE_INT32_T, 0, 20, offsetof(mavlink_cc_telemetry_gps_t, alt) }, \
         { "eph", NULL, MAVLINK_TYPE_FLOAT, 0, 24, offsetof(mavlink_cc_telemetry_gps_t, eph) }, \
         { "epv", NULL, MAVLINK_TYPE_FLOAT, 0, 28, offsetof(mavlink_cc_telemetry_gps_t, epv) }, \
         { "ground_speed", NULL, MAVLINK_TYPE_FLOAT, 0, 32, offsetof(mavlink_cc_telemetry_gps_t, ground_speed) }, \
         { "heading", NULL, MAVLINK_TYPE_FLOAT, 0, 36, offsetof(mavlink_cc_telemetry_gps_t, heading) }, \
         { "noise_per_ms", NULL, MAVLINK_TYPE_UINT16_T, 0, 40, offsetof(mavlink_cc_telemetry_gps_t, noise_per_ms) }, \
         { "jamming_indicator", NULL, MAVLINK_TYPE_UINT16_T, 0, 42, offsetof(mavlink_cc_telemetry_gps_t, jamming_indicator) }, \
         { "fix_type", NULL, MAVLINK_TYPE_UINT8_T, 0, 44, offsetof(mavlink_cc_telemetry_gps_t, fix_type) }, \
         { "satellites_used", NULL, MAVLINK_TYPE_UINT8_T, 0, 45, offsetof(mavlink_cc_telemetry_gps_t, satellites_used) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 46, offsetof(mavlink_cc_telemetry_gps_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_GPS { \
    "CC_TELEMETRY_GPS", \
    14, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_gps_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_gps_t, sequence) }, \
         { "lat", NULL, MAVLINK_TYPE_INT32_T, 0, 12, offsetof(mavlink_cc_telemetry_gps_t, lat) }, \
         { "lon", NULL, MAVLINK_TYPE_INT32_T, 0, 16, offsetof(mavlink_cc_telemetry_gps_t, lon) }, \
         { "alt", NULL, MAVLINK_TYPE_INT32_T, 0, 20, offsetof(mavlink_cc_telemetry_gps_t, alt) }, \
         { "eph", NULL, MAVLINK_TYPE_FLOAT, 0, 24, offsetof(mavlink_cc_telemetry_gps_t, eph) }, \
         { "epv", NULL, MAVLINK_TYPE_FLOAT, 0, 28, offsetof(mavlink_cc_telemetry_gps_t, epv) }, \
         { "ground_speed", NULL, MAVLINK_TYPE_FLOAT, 0, 32, offsetof(mavlink_cc_telemetry_gps_t, ground_speed) }, \
         { "heading", NULL, MAVLINK_TYPE_FLOAT, 0, 36, offsetof(mavlink_cc_telemetry_gps_t, heading) }, \
         { "noise_per_ms", NULL, MAVLINK_TYPE_UINT16_T, 0, 40, offsetof(mavlink_cc_telemetry_gps_t, noise_per_ms) }, \
         { "jamming_indicator", NULL, MAVLINK_TYPE_UINT16_T, 0, 42, offsetof(mavlink_cc_telemetry_gps_t, jamming_indicator) }, \
         { "fix_type", NULL, MAVLINK_TYPE_UINT8_T, 0, 44, offsetof(mavlink_cc_telemetry_gps_t, fix_type) }, \
         { "satellites_used", NULL, MAVLINK_TYPE_UINT8_T, 0, 45, offsetof(mavlink_cc_telemetry_gps_t, satellites_used) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 46, offsetof(mavlink_cc_telemetry_gps_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_telemetry_gps message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param lat [degE7] Latitude, WGS-84, degrees * 1e7.
 * @param lon [degE7] Longitude, WGS-84, degrees * 1e7.
 * @param alt [mm] Altitude AMSL, millimeters.
 * @param eph [m] Horizontal position uncertainty.
 * @param epv [m] Vertical position uncertainty.
 * @param ground_speed [m/s] Ground speed.
 * @param heading [rad] Course over ground, [-pi, pi] (NaN if unknown).
 * @param noise_per_ms  GPS noise indicator (receiver-specific; 0 if unavailable).
 * @param jamming_indicator  GPS jamming indicator (receiver-specific; 0 if unavailable).
 * @param fix_type  GNSS fix type (as PX4 sensor_gps fix_type).
 * @param satellites_used  Satellites used in solution.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_gps_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, int32_t lat, int32_t lon, int32_t alt, float eph, float epv, float ground_speed, float heading, uint16_t noise_per_ms, uint16_t jamming_indicator, uint8_t fix_type, uint8_t satellites_used, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_int32_t(buf, 12, lat);
    _mav_put_int32_t(buf, 16, lon);
    _mav_put_int32_t(buf, 20, alt);
    _mav_put_float(buf, 24, eph);
    _mav_put_float(buf, 28, epv);
    _mav_put_float(buf, 32, ground_speed);
    _mav_put_float(buf, 36, heading);
    _mav_put_uint16_t(buf, 40, noise_per_ms);
    _mav_put_uint16_t(buf, 42, jamming_indicator);
    _mav_put_uint8_t(buf, 44, fix_type);
    _mav_put_uint8_t(buf, 45, satellites_used);
    _mav_put_uint8_t(buf, 46, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN);
#else
    mavlink_cc_telemetry_gps_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.lat = lat;
    packet.lon = lon;
    packet.alt = alt;
    packet.eph = eph;
    packet.epv = epv;
    packet.ground_speed = ground_speed;
    packet.heading = heading;
    packet.noise_per_ms = noise_per_ms;
    packet.jamming_indicator = jamming_indicator;
    packet.fix_type = fix_type;
    packet.satellites_used = satellites_used;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_GPS;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_CRC);
}

/**
 * @brief Pack a cc_telemetry_gps message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param lat [degE7] Latitude, WGS-84, degrees * 1e7.
 * @param lon [degE7] Longitude, WGS-84, degrees * 1e7.
 * @param alt [mm] Altitude AMSL, millimeters.
 * @param eph [m] Horizontal position uncertainty.
 * @param epv [m] Vertical position uncertainty.
 * @param ground_speed [m/s] Ground speed.
 * @param heading [rad] Course over ground, [-pi, pi] (NaN if unknown).
 * @param noise_per_ms  GPS noise indicator (receiver-specific; 0 if unavailable).
 * @param jamming_indicator  GPS jamming indicator (receiver-specific; 0 if unavailable).
 * @param fix_type  GNSS fix type (as PX4 sensor_gps fix_type).
 * @param satellites_used  Satellites used in solution.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_gps_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, int32_t lat, int32_t lon, int32_t alt, float eph, float epv, float ground_speed, float heading, uint16_t noise_per_ms, uint16_t jamming_indicator, uint8_t fix_type, uint8_t satellites_used, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_int32_t(buf, 12, lat);
    _mav_put_int32_t(buf, 16, lon);
    _mav_put_int32_t(buf, 20, alt);
    _mav_put_float(buf, 24, eph);
    _mav_put_float(buf, 28, epv);
    _mav_put_float(buf, 32, ground_speed);
    _mav_put_float(buf, 36, heading);
    _mav_put_uint16_t(buf, 40, noise_per_ms);
    _mav_put_uint16_t(buf, 42, jamming_indicator);
    _mav_put_uint8_t(buf, 44, fix_type);
    _mav_put_uint8_t(buf, 45, satellites_used);
    _mav_put_uint8_t(buf, 46, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN);
#else
    mavlink_cc_telemetry_gps_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.lat = lat;
    packet.lon = lon;
    packet.alt = alt;
    packet.eph = eph;
    packet.epv = epv;
    packet.ground_speed = ground_speed;
    packet.heading = heading;
    packet.noise_per_ms = noise_per_ms;
    packet.jamming_indicator = jamming_indicator;
    packet.fix_type = fix_type;
    packet.satellites_used = satellites_used;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_GPS;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN);
#endif
}

/**
 * @brief Pack a cc_telemetry_gps message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param lat [degE7] Latitude, WGS-84, degrees * 1e7.
 * @param lon [degE7] Longitude, WGS-84, degrees * 1e7.
 * @param alt [mm] Altitude AMSL, millimeters.
 * @param eph [m] Horizontal position uncertainty.
 * @param epv [m] Vertical position uncertainty.
 * @param ground_speed [m/s] Ground speed.
 * @param heading [rad] Course over ground, [-pi, pi] (NaN if unknown).
 * @param noise_per_ms  GPS noise indicator (receiver-specific; 0 if unavailable).
 * @param jamming_indicator  GPS jamming indicator (receiver-specific; 0 if unavailable).
 * @param fix_type  GNSS fix type (as PX4 sensor_gps fix_type).
 * @param satellites_used  Satellites used in solution.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_gps_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t fc_timestamp_us,uint32_t sequence,int32_t lat,int32_t lon,int32_t alt,float eph,float epv,float ground_speed,float heading,uint16_t noise_per_ms,uint16_t jamming_indicator,uint8_t fix_type,uint8_t satellites_used,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_int32_t(buf, 12, lat);
    _mav_put_int32_t(buf, 16, lon);
    _mav_put_int32_t(buf, 20, alt);
    _mav_put_float(buf, 24, eph);
    _mav_put_float(buf, 28, epv);
    _mav_put_float(buf, 32, ground_speed);
    _mav_put_float(buf, 36, heading);
    _mav_put_uint16_t(buf, 40, noise_per_ms);
    _mav_put_uint16_t(buf, 42, jamming_indicator);
    _mav_put_uint8_t(buf, 44, fix_type);
    _mav_put_uint8_t(buf, 45, satellites_used);
    _mav_put_uint8_t(buf, 46, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN);
#else
    mavlink_cc_telemetry_gps_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.lat = lat;
    packet.lon = lon;
    packet.alt = alt;
    packet.eph = eph;
    packet.epv = epv;
    packet.ground_speed = ground_speed;
    packet.heading = heading;
    packet.noise_per_ms = noise_per_ms;
    packet.jamming_indicator = jamming_indicator;
    packet.fix_type = fix_type;
    packet.satellites_used = satellites_used;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_GPS;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_CRC);
}

/**
 * @brief Encode a cc_telemetry_gps struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_gps C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_gps_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_telemetry_gps_t* cc_telemetry_gps)
{
    return mavlink_msg_cc_telemetry_gps_pack(system_id, component_id, msg, cc_telemetry_gps->fc_timestamp_us, cc_telemetry_gps->sequence, cc_telemetry_gps->lat, cc_telemetry_gps->lon, cc_telemetry_gps->alt, cc_telemetry_gps->eph, cc_telemetry_gps->epv, cc_telemetry_gps->ground_speed, cc_telemetry_gps->heading, cc_telemetry_gps->noise_per_ms, cc_telemetry_gps->jamming_indicator, cc_telemetry_gps->fix_type, cc_telemetry_gps->satellites_used, cc_telemetry_gps->schema_version);
}

/**
 * @brief Encode a cc_telemetry_gps struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_gps C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_gps_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_telemetry_gps_t* cc_telemetry_gps)
{
    return mavlink_msg_cc_telemetry_gps_pack_chan(system_id, component_id, chan, msg, cc_telemetry_gps->fc_timestamp_us, cc_telemetry_gps->sequence, cc_telemetry_gps->lat, cc_telemetry_gps->lon, cc_telemetry_gps->alt, cc_telemetry_gps->eph, cc_telemetry_gps->epv, cc_telemetry_gps->ground_speed, cc_telemetry_gps->heading, cc_telemetry_gps->noise_per_ms, cc_telemetry_gps->jamming_indicator, cc_telemetry_gps->fix_type, cc_telemetry_gps->satellites_used, cc_telemetry_gps->schema_version);
}

/**
 * @brief Encode a cc_telemetry_gps struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_gps C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_gps_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_telemetry_gps_t* cc_telemetry_gps)
{
    return mavlink_msg_cc_telemetry_gps_pack_status(system_id, component_id, _status, msg,  cc_telemetry_gps->fc_timestamp_us, cc_telemetry_gps->sequence, cc_telemetry_gps->lat, cc_telemetry_gps->lon, cc_telemetry_gps->alt, cc_telemetry_gps->eph, cc_telemetry_gps->epv, cc_telemetry_gps->ground_speed, cc_telemetry_gps->heading, cc_telemetry_gps->noise_per_ms, cc_telemetry_gps->jamming_indicator, cc_telemetry_gps->fix_type, cc_telemetry_gps->satellites_used, cc_telemetry_gps->schema_version);
}

/**
 * @brief Send a cc_telemetry_gps message
 * @param chan MAVLink channel to send the message
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param lat [degE7] Latitude, WGS-84, degrees * 1e7.
 * @param lon [degE7] Longitude, WGS-84, degrees * 1e7.
 * @param alt [mm] Altitude AMSL, millimeters.
 * @param eph [m] Horizontal position uncertainty.
 * @param epv [m] Vertical position uncertainty.
 * @param ground_speed [m/s] Ground speed.
 * @param heading [rad] Course over ground, [-pi, pi] (NaN if unknown).
 * @param noise_per_ms  GPS noise indicator (receiver-specific; 0 if unavailable).
 * @param jamming_indicator  GPS jamming indicator (receiver-specific; 0 if unavailable).
 * @param fix_type  GNSS fix type (as PX4 sensor_gps fix_type).
 * @param satellites_used  Satellites used in solution.
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_telemetry_gps_send(mavlink_channel_t chan, uint64_t fc_timestamp_us, uint32_t sequence, int32_t lat, int32_t lon, int32_t alt, float eph, float epv, float ground_speed, float heading, uint16_t noise_per_ms, uint16_t jamming_indicator, uint8_t fix_type, uint8_t satellites_used, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_int32_t(buf, 12, lat);
    _mav_put_int32_t(buf, 16, lon);
    _mav_put_int32_t(buf, 20, alt);
    _mav_put_float(buf, 24, eph);
    _mav_put_float(buf, 28, epv);
    _mav_put_float(buf, 32, ground_speed);
    _mav_put_float(buf, 36, heading);
    _mav_put_uint16_t(buf, 40, noise_per_ms);
    _mav_put_uint16_t(buf, 42, jamming_indicator);
    _mav_put_uint8_t(buf, 44, fix_type);
    _mav_put_uint8_t(buf, 45, satellites_used);
    _mav_put_uint8_t(buf, 46, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_GPS, buf, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_CRC);
#else
    mavlink_cc_telemetry_gps_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.lat = lat;
    packet.lon = lon;
    packet.alt = alt;
    packet.eph = eph;
    packet.epv = epv;
    packet.ground_speed = ground_speed;
    packet.heading = heading;
    packet.noise_per_ms = noise_per_ms;
    packet.jamming_indicator = jamming_indicator;
    packet.fix_type = fix_type;
    packet.satellites_used = satellites_used;
    packet.schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_GPS, (const char *)&packet, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_CRC);
#endif
}

/**
 * @brief Send a cc_telemetry_gps message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_telemetry_gps_send_struct(mavlink_channel_t chan, const mavlink_cc_telemetry_gps_t* cc_telemetry_gps)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_telemetry_gps_send(chan, cc_telemetry_gps->fc_timestamp_us, cc_telemetry_gps->sequence, cc_telemetry_gps->lat, cc_telemetry_gps->lon, cc_telemetry_gps->alt, cc_telemetry_gps->eph, cc_telemetry_gps->epv, cc_telemetry_gps->ground_speed, cc_telemetry_gps->heading, cc_telemetry_gps->noise_per_ms, cc_telemetry_gps->jamming_indicator, cc_telemetry_gps->fix_type, cc_telemetry_gps->satellites_used, cc_telemetry_gps->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_GPS, (const char *)cc_telemetry_gps, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_telemetry_gps_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t fc_timestamp_us, uint32_t sequence, int32_t lat, int32_t lon, int32_t alt, float eph, float epv, float ground_speed, float heading, uint16_t noise_per_ms, uint16_t jamming_indicator, uint8_t fix_type, uint8_t satellites_used, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_int32_t(buf, 12, lat);
    _mav_put_int32_t(buf, 16, lon);
    _mav_put_int32_t(buf, 20, alt);
    _mav_put_float(buf, 24, eph);
    _mav_put_float(buf, 28, epv);
    _mav_put_float(buf, 32, ground_speed);
    _mav_put_float(buf, 36, heading);
    _mav_put_uint16_t(buf, 40, noise_per_ms);
    _mav_put_uint16_t(buf, 42, jamming_indicator);
    _mav_put_uint8_t(buf, 44, fix_type);
    _mav_put_uint8_t(buf, 45, satellites_used);
    _mav_put_uint8_t(buf, 46, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_GPS, buf, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_CRC);
#else
    mavlink_cc_telemetry_gps_t *packet = (mavlink_cc_telemetry_gps_t *)msgbuf;
    packet->fc_timestamp_us = fc_timestamp_us;
    packet->sequence = sequence;
    packet->lat = lat;
    packet->lon = lon;
    packet->alt = alt;
    packet->eph = eph;
    packet->epv = epv;
    packet->ground_speed = ground_speed;
    packet->heading = heading;
    packet->noise_per_ms = noise_per_ms;
    packet->jamming_indicator = jamming_indicator;
    packet->fix_type = fix_type;
    packet->satellites_used = satellites_used;
    packet->schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_GPS, (const char *)packet, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_TELEMETRY_GPS UNPACKING


/**
 * @brief Get field fc_timestamp_us from cc_telemetry_gps message
 *
 * @return [us] FC monotonic time since PX4 boot.
 */
static inline uint64_t mavlink_msg_cc_telemetry_gps_get_fc_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_telemetry_gps message
 *
 * @return  Per-stream monotonic counter.
 */
static inline uint32_t mavlink_msg_cc_telemetry_gps_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field lat from cc_telemetry_gps message
 *
 * @return [degE7] Latitude, WGS-84, degrees * 1e7.
 */
static inline int32_t mavlink_msg_cc_telemetry_gps_get_lat(const mavlink_message_t* msg)
{
    return _MAV_RETURN_int32_t(msg,  12);
}

/**
 * @brief Get field lon from cc_telemetry_gps message
 *
 * @return [degE7] Longitude, WGS-84, degrees * 1e7.
 */
static inline int32_t mavlink_msg_cc_telemetry_gps_get_lon(const mavlink_message_t* msg)
{
    return _MAV_RETURN_int32_t(msg,  16);
}

/**
 * @brief Get field alt from cc_telemetry_gps message
 *
 * @return [mm] Altitude AMSL, millimeters.
 */
static inline int32_t mavlink_msg_cc_telemetry_gps_get_alt(const mavlink_message_t* msg)
{
    return _MAV_RETURN_int32_t(msg,  20);
}

/**
 * @brief Get field eph from cc_telemetry_gps message
 *
 * @return [m] Horizontal position uncertainty.
 */
static inline float mavlink_msg_cc_telemetry_gps_get_eph(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  24);
}

/**
 * @brief Get field epv from cc_telemetry_gps message
 *
 * @return [m] Vertical position uncertainty.
 */
static inline float mavlink_msg_cc_telemetry_gps_get_epv(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  28);
}

/**
 * @brief Get field ground_speed from cc_telemetry_gps message
 *
 * @return [m/s] Ground speed.
 */
static inline float mavlink_msg_cc_telemetry_gps_get_ground_speed(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  32);
}

/**
 * @brief Get field heading from cc_telemetry_gps message
 *
 * @return [rad] Course over ground, [-pi, pi] (NaN if unknown).
 */
static inline float mavlink_msg_cc_telemetry_gps_get_heading(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  36);
}

/**
 * @brief Get field noise_per_ms from cc_telemetry_gps message
 *
 * @return  GPS noise indicator (receiver-specific; 0 if unavailable).
 */
static inline uint16_t mavlink_msg_cc_telemetry_gps_get_noise_per_ms(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  40);
}

/**
 * @brief Get field jamming_indicator from cc_telemetry_gps message
 *
 * @return  GPS jamming indicator (receiver-specific; 0 if unavailable).
 */
static inline uint16_t mavlink_msg_cc_telemetry_gps_get_jamming_indicator(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  42);
}

/**
 * @brief Get field fix_type from cc_telemetry_gps message
 *
 * @return  GNSS fix type (as PX4 sensor_gps fix_type).
 */
static inline uint8_t mavlink_msg_cc_telemetry_gps_get_fix_type(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  44);
}

/**
 * @brief Get field satellites_used from cc_telemetry_gps message
 *
 * @return  Satellites used in solution.
 */
static inline uint8_t mavlink_msg_cc_telemetry_gps_get_satellites_used(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  45);
}

/**
 * @brief Get field schema_version from cc_telemetry_gps message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_telemetry_gps_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  46);
}

/**
 * @brief Decode a cc_telemetry_gps message into a struct
 *
 * @param msg The message to decode
 * @param cc_telemetry_gps C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_telemetry_gps_decode(const mavlink_message_t* msg, mavlink_cc_telemetry_gps_t* cc_telemetry_gps)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_telemetry_gps->fc_timestamp_us = mavlink_msg_cc_telemetry_gps_get_fc_timestamp_us(msg);
    cc_telemetry_gps->sequence = mavlink_msg_cc_telemetry_gps_get_sequence(msg);
    cc_telemetry_gps->lat = mavlink_msg_cc_telemetry_gps_get_lat(msg);
    cc_telemetry_gps->lon = mavlink_msg_cc_telemetry_gps_get_lon(msg);
    cc_telemetry_gps->alt = mavlink_msg_cc_telemetry_gps_get_alt(msg);
    cc_telemetry_gps->eph = mavlink_msg_cc_telemetry_gps_get_eph(msg);
    cc_telemetry_gps->epv = mavlink_msg_cc_telemetry_gps_get_epv(msg);
    cc_telemetry_gps->ground_speed = mavlink_msg_cc_telemetry_gps_get_ground_speed(msg);
    cc_telemetry_gps->heading = mavlink_msg_cc_telemetry_gps_get_heading(msg);
    cc_telemetry_gps->noise_per_ms = mavlink_msg_cc_telemetry_gps_get_noise_per_ms(msg);
    cc_telemetry_gps->jamming_indicator = mavlink_msg_cc_telemetry_gps_get_jamming_indicator(msg);
    cc_telemetry_gps->fix_type = mavlink_msg_cc_telemetry_gps_get_fix_type(msg);
    cc_telemetry_gps->satellites_used = mavlink_msg_cc_telemetry_gps_get_satellites_used(msg);
    cc_telemetry_gps->schema_version = mavlink_msg_cc_telemetry_gps_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN? msg->len : MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN;
        memset(cc_telemetry_gps, 0, MAVLINK_MSG_ID_CC_TELEMETRY_GPS_LEN);
    memcpy(cc_telemetry_gps, _MAV_PAYLOAD(msg), len);
#endif
}
