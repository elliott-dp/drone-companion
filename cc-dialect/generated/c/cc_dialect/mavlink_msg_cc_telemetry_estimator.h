#pragma once
// MESSAGE CC_TELEMETRY_ESTIMATOR PACKING

#define MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR 54004


typedef struct __mavlink_cc_telemetry_estimator_t {
 uint64_t fc_timestamp_us; /*< [us] FC monotonic time since PX4 boot.*/
 uint32_t sequence; /*<  Per-stream monotonic counter.*/
 uint32_t status_flags; /*<  EKF status/fault flags (compacted from estimator_status).*/
 float velocity_test_ratio; /*<  Velocity innovation test ratio (<1 = passing).*/
 float position_test_ratio; /*<  Horizontal position innovation test ratio.*/
 float height_test_ratio; /*<  Height innovation test ratio.*/
 float mag_test_ratio; /*<  Magnetometer innovation test ratio.*/
 float airspeed_test_ratio; /*<  Airspeed innovation test ratio (NaN if not applicable).*/
 uint16_t innovation_check_flags; /*<  EKF innovation check failure bitfield.*/
 uint16_t solution_status_flags; /*<  EKF solution status bitfield.*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_telemetry_estimator_t;

#define MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN 41
#define MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN 41
#define MAVLINK_MSG_ID_54004_LEN 41
#define MAVLINK_MSG_ID_54004_MIN_LEN 41

#define MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_CRC 190
#define MAVLINK_MSG_ID_54004_CRC 190



#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_ESTIMATOR { \
    54004, \
    "CC_TELEMETRY_ESTIMATOR", \
    11, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_estimator_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_estimator_t, sequence) }, \
         { "status_flags", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_telemetry_estimator_t, status_flags) }, \
         { "innovation_check_flags", NULL, MAVLINK_TYPE_UINT16_T, 0, 36, offsetof(mavlink_cc_telemetry_estimator_t, innovation_check_flags) }, \
         { "solution_status_flags", NULL, MAVLINK_TYPE_UINT16_T, 0, 38, offsetof(mavlink_cc_telemetry_estimator_t, solution_status_flags) }, \
         { "velocity_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 16, offsetof(mavlink_cc_telemetry_estimator_t, velocity_test_ratio) }, \
         { "position_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 20, offsetof(mavlink_cc_telemetry_estimator_t, position_test_ratio) }, \
         { "height_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 24, offsetof(mavlink_cc_telemetry_estimator_t, height_test_ratio) }, \
         { "mag_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 28, offsetof(mavlink_cc_telemetry_estimator_t, mag_test_ratio) }, \
         { "airspeed_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 32, offsetof(mavlink_cc_telemetry_estimator_t, airspeed_test_ratio) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 40, offsetof(mavlink_cc_telemetry_estimator_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_ESTIMATOR { \
    "CC_TELEMETRY_ESTIMATOR", \
    11, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_estimator_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_estimator_t, sequence) }, \
         { "status_flags", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_telemetry_estimator_t, status_flags) }, \
         { "innovation_check_flags", NULL, MAVLINK_TYPE_UINT16_T, 0, 36, offsetof(mavlink_cc_telemetry_estimator_t, innovation_check_flags) }, \
         { "solution_status_flags", NULL, MAVLINK_TYPE_UINT16_T, 0, 38, offsetof(mavlink_cc_telemetry_estimator_t, solution_status_flags) }, \
         { "velocity_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 16, offsetof(mavlink_cc_telemetry_estimator_t, velocity_test_ratio) }, \
         { "position_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 20, offsetof(mavlink_cc_telemetry_estimator_t, position_test_ratio) }, \
         { "height_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 24, offsetof(mavlink_cc_telemetry_estimator_t, height_test_ratio) }, \
         { "mag_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 28, offsetof(mavlink_cc_telemetry_estimator_t, mag_test_ratio) }, \
         { "airspeed_test_ratio", NULL, MAVLINK_TYPE_FLOAT, 0, 32, offsetof(mavlink_cc_telemetry_estimator_t, airspeed_test_ratio) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 40, offsetof(mavlink_cc_telemetry_estimator_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_telemetry_estimator message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param status_flags  EKF status/fault flags (compacted from estimator_status).
 * @param innovation_check_flags  EKF innovation check failure bitfield.
 * @param solution_status_flags  EKF solution status bitfield.
 * @param velocity_test_ratio  Velocity innovation test ratio (<1 = passing).
 * @param position_test_ratio  Horizontal position innovation test ratio.
 * @param height_test_ratio  Height innovation test ratio.
 * @param mag_test_ratio  Magnetometer innovation test ratio.
 * @param airspeed_test_ratio  Airspeed innovation test ratio (NaN if not applicable).
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_estimator_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, uint32_t status_flags, uint16_t innovation_check_flags, uint16_t solution_status_flags, float velocity_test_ratio, float position_test_ratio, float height_test_ratio, float mag_test_ratio, float airspeed_test_ratio, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, status_flags);
    _mav_put_float(buf, 16, velocity_test_ratio);
    _mav_put_float(buf, 20, position_test_ratio);
    _mav_put_float(buf, 24, height_test_ratio);
    _mav_put_float(buf, 28, mag_test_ratio);
    _mav_put_float(buf, 32, airspeed_test_ratio);
    _mav_put_uint16_t(buf, 36, innovation_check_flags);
    _mav_put_uint16_t(buf, 38, solution_status_flags);
    _mav_put_uint8_t(buf, 40, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN);
#else
    mavlink_cc_telemetry_estimator_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.status_flags = status_flags;
    packet.velocity_test_ratio = velocity_test_ratio;
    packet.position_test_ratio = position_test_ratio;
    packet.height_test_ratio = height_test_ratio;
    packet.mag_test_ratio = mag_test_ratio;
    packet.airspeed_test_ratio = airspeed_test_ratio;
    packet.innovation_check_flags = innovation_check_flags;
    packet.solution_status_flags = solution_status_flags;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_CRC);
}

/**
 * @brief Pack a cc_telemetry_estimator message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param status_flags  EKF status/fault flags (compacted from estimator_status).
 * @param innovation_check_flags  EKF innovation check failure bitfield.
 * @param solution_status_flags  EKF solution status bitfield.
 * @param velocity_test_ratio  Velocity innovation test ratio (<1 = passing).
 * @param position_test_ratio  Horizontal position innovation test ratio.
 * @param height_test_ratio  Height innovation test ratio.
 * @param mag_test_ratio  Magnetometer innovation test ratio.
 * @param airspeed_test_ratio  Airspeed innovation test ratio (NaN if not applicable).
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_estimator_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, uint32_t status_flags, uint16_t innovation_check_flags, uint16_t solution_status_flags, float velocity_test_ratio, float position_test_ratio, float height_test_ratio, float mag_test_ratio, float airspeed_test_ratio, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, status_flags);
    _mav_put_float(buf, 16, velocity_test_ratio);
    _mav_put_float(buf, 20, position_test_ratio);
    _mav_put_float(buf, 24, height_test_ratio);
    _mav_put_float(buf, 28, mag_test_ratio);
    _mav_put_float(buf, 32, airspeed_test_ratio);
    _mav_put_uint16_t(buf, 36, innovation_check_flags);
    _mav_put_uint16_t(buf, 38, solution_status_flags);
    _mav_put_uint8_t(buf, 40, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN);
#else
    mavlink_cc_telemetry_estimator_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.status_flags = status_flags;
    packet.velocity_test_ratio = velocity_test_ratio;
    packet.position_test_ratio = position_test_ratio;
    packet.height_test_ratio = height_test_ratio;
    packet.mag_test_ratio = mag_test_ratio;
    packet.airspeed_test_ratio = airspeed_test_ratio;
    packet.innovation_check_flags = innovation_check_flags;
    packet.solution_status_flags = solution_status_flags;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN);
#endif
}

/**
 * @brief Pack a cc_telemetry_estimator message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param status_flags  EKF status/fault flags (compacted from estimator_status).
 * @param innovation_check_flags  EKF innovation check failure bitfield.
 * @param solution_status_flags  EKF solution status bitfield.
 * @param velocity_test_ratio  Velocity innovation test ratio (<1 = passing).
 * @param position_test_ratio  Horizontal position innovation test ratio.
 * @param height_test_ratio  Height innovation test ratio.
 * @param mag_test_ratio  Magnetometer innovation test ratio.
 * @param airspeed_test_ratio  Airspeed innovation test ratio (NaN if not applicable).
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_estimator_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t fc_timestamp_us,uint32_t sequence,uint32_t status_flags,uint16_t innovation_check_flags,uint16_t solution_status_flags,float velocity_test_ratio,float position_test_ratio,float height_test_ratio,float mag_test_ratio,float airspeed_test_ratio,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, status_flags);
    _mav_put_float(buf, 16, velocity_test_ratio);
    _mav_put_float(buf, 20, position_test_ratio);
    _mav_put_float(buf, 24, height_test_ratio);
    _mav_put_float(buf, 28, mag_test_ratio);
    _mav_put_float(buf, 32, airspeed_test_ratio);
    _mav_put_uint16_t(buf, 36, innovation_check_flags);
    _mav_put_uint16_t(buf, 38, solution_status_flags);
    _mav_put_uint8_t(buf, 40, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN);
#else
    mavlink_cc_telemetry_estimator_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.status_flags = status_flags;
    packet.velocity_test_ratio = velocity_test_ratio;
    packet.position_test_ratio = position_test_ratio;
    packet.height_test_ratio = height_test_ratio;
    packet.mag_test_ratio = mag_test_ratio;
    packet.airspeed_test_ratio = airspeed_test_ratio;
    packet.innovation_check_flags = innovation_check_flags;
    packet.solution_status_flags = solution_status_flags;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_CRC);
}

/**
 * @brief Encode a cc_telemetry_estimator struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_estimator C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_estimator_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_telemetry_estimator_t* cc_telemetry_estimator)
{
    return mavlink_msg_cc_telemetry_estimator_pack(system_id, component_id, msg, cc_telemetry_estimator->fc_timestamp_us, cc_telemetry_estimator->sequence, cc_telemetry_estimator->status_flags, cc_telemetry_estimator->innovation_check_flags, cc_telemetry_estimator->solution_status_flags, cc_telemetry_estimator->velocity_test_ratio, cc_telemetry_estimator->position_test_ratio, cc_telemetry_estimator->height_test_ratio, cc_telemetry_estimator->mag_test_ratio, cc_telemetry_estimator->airspeed_test_ratio, cc_telemetry_estimator->schema_version);
}

/**
 * @brief Encode a cc_telemetry_estimator struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_estimator C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_estimator_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_telemetry_estimator_t* cc_telemetry_estimator)
{
    return mavlink_msg_cc_telemetry_estimator_pack_chan(system_id, component_id, chan, msg, cc_telemetry_estimator->fc_timestamp_us, cc_telemetry_estimator->sequence, cc_telemetry_estimator->status_flags, cc_telemetry_estimator->innovation_check_flags, cc_telemetry_estimator->solution_status_flags, cc_telemetry_estimator->velocity_test_ratio, cc_telemetry_estimator->position_test_ratio, cc_telemetry_estimator->height_test_ratio, cc_telemetry_estimator->mag_test_ratio, cc_telemetry_estimator->airspeed_test_ratio, cc_telemetry_estimator->schema_version);
}

/**
 * @brief Encode a cc_telemetry_estimator struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_estimator C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_estimator_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_telemetry_estimator_t* cc_telemetry_estimator)
{
    return mavlink_msg_cc_telemetry_estimator_pack_status(system_id, component_id, _status, msg,  cc_telemetry_estimator->fc_timestamp_us, cc_telemetry_estimator->sequence, cc_telemetry_estimator->status_flags, cc_telemetry_estimator->innovation_check_flags, cc_telemetry_estimator->solution_status_flags, cc_telemetry_estimator->velocity_test_ratio, cc_telemetry_estimator->position_test_ratio, cc_telemetry_estimator->height_test_ratio, cc_telemetry_estimator->mag_test_ratio, cc_telemetry_estimator->airspeed_test_ratio, cc_telemetry_estimator->schema_version);
}

/**
 * @brief Send a cc_telemetry_estimator message
 * @param chan MAVLink channel to send the message
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param status_flags  EKF status/fault flags (compacted from estimator_status).
 * @param innovation_check_flags  EKF innovation check failure bitfield.
 * @param solution_status_flags  EKF solution status bitfield.
 * @param velocity_test_ratio  Velocity innovation test ratio (<1 = passing).
 * @param position_test_ratio  Horizontal position innovation test ratio.
 * @param height_test_ratio  Height innovation test ratio.
 * @param mag_test_ratio  Magnetometer innovation test ratio.
 * @param airspeed_test_ratio  Airspeed innovation test ratio (NaN if not applicable).
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_telemetry_estimator_send(mavlink_channel_t chan, uint64_t fc_timestamp_us, uint32_t sequence, uint32_t status_flags, uint16_t innovation_check_flags, uint16_t solution_status_flags, float velocity_test_ratio, float position_test_ratio, float height_test_ratio, float mag_test_ratio, float airspeed_test_ratio, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, status_flags);
    _mav_put_float(buf, 16, velocity_test_ratio);
    _mav_put_float(buf, 20, position_test_ratio);
    _mav_put_float(buf, 24, height_test_ratio);
    _mav_put_float(buf, 28, mag_test_ratio);
    _mav_put_float(buf, 32, airspeed_test_ratio);
    _mav_put_uint16_t(buf, 36, innovation_check_flags);
    _mav_put_uint16_t(buf, 38, solution_status_flags);
    _mav_put_uint8_t(buf, 40, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR, buf, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_CRC);
#else
    mavlink_cc_telemetry_estimator_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.status_flags = status_flags;
    packet.velocity_test_ratio = velocity_test_ratio;
    packet.position_test_ratio = position_test_ratio;
    packet.height_test_ratio = height_test_ratio;
    packet.mag_test_ratio = mag_test_ratio;
    packet.airspeed_test_ratio = airspeed_test_ratio;
    packet.innovation_check_flags = innovation_check_flags;
    packet.solution_status_flags = solution_status_flags;
    packet.schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR, (const char *)&packet, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_CRC);
#endif
}

/**
 * @brief Send a cc_telemetry_estimator message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_telemetry_estimator_send_struct(mavlink_channel_t chan, const mavlink_cc_telemetry_estimator_t* cc_telemetry_estimator)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_telemetry_estimator_send(chan, cc_telemetry_estimator->fc_timestamp_us, cc_telemetry_estimator->sequence, cc_telemetry_estimator->status_flags, cc_telemetry_estimator->innovation_check_flags, cc_telemetry_estimator->solution_status_flags, cc_telemetry_estimator->velocity_test_ratio, cc_telemetry_estimator->position_test_ratio, cc_telemetry_estimator->height_test_ratio, cc_telemetry_estimator->mag_test_ratio, cc_telemetry_estimator->airspeed_test_ratio, cc_telemetry_estimator->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR, (const char *)cc_telemetry_estimator, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_telemetry_estimator_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t fc_timestamp_us, uint32_t sequence, uint32_t status_flags, uint16_t innovation_check_flags, uint16_t solution_status_flags, float velocity_test_ratio, float position_test_ratio, float height_test_ratio, float mag_test_ratio, float airspeed_test_ratio, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, status_flags);
    _mav_put_float(buf, 16, velocity_test_ratio);
    _mav_put_float(buf, 20, position_test_ratio);
    _mav_put_float(buf, 24, height_test_ratio);
    _mav_put_float(buf, 28, mag_test_ratio);
    _mav_put_float(buf, 32, airspeed_test_ratio);
    _mav_put_uint16_t(buf, 36, innovation_check_flags);
    _mav_put_uint16_t(buf, 38, solution_status_flags);
    _mav_put_uint8_t(buf, 40, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR, buf, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_CRC);
#else
    mavlink_cc_telemetry_estimator_t *packet = (mavlink_cc_telemetry_estimator_t *)msgbuf;
    packet->fc_timestamp_us = fc_timestamp_us;
    packet->sequence = sequence;
    packet->status_flags = status_flags;
    packet->velocity_test_ratio = velocity_test_ratio;
    packet->position_test_ratio = position_test_ratio;
    packet->height_test_ratio = height_test_ratio;
    packet->mag_test_ratio = mag_test_ratio;
    packet->airspeed_test_ratio = airspeed_test_ratio;
    packet->innovation_check_flags = innovation_check_flags;
    packet->solution_status_flags = solution_status_flags;
    packet->schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR, (const char *)packet, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_TELEMETRY_ESTIMATOR UNPACKING


/**
 * @brief Get field fc_timestamp_us from cc_telemetry_estimator message
 *
 * @return [us] FC monotonic time since PX4 boot.
 */
static inline uint64_t mavlink_msg_cc_telemetry_estimator_get_fc_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_telemetry_estimator message
 *
 * @return  Per-stream monotonic counter.
 */
static inline uint32_t mavlink_msg_cc_telemetry_estimator_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field status_flags from cc_telemetry_estimator message
 *
 * @return  EKF status/fault flags (compacted from estimator_status).
 */
static inline uint32_t mavlink_msg_cc_telemetry_estimator_get_status_flags(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  12);
}

/**
 * @brief Get field innovation_check_flags from cc_telemetry_estimator message
 *
 * @return  EKF innovation check failure bitfield.
 */
static inline uint16_t mavlink_msg_cc_telemetry_estimator_get_innovation_check_flags(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  36);
}

/**
 * @brief Get field solution_status_flags from cc_telemetry_estimator message
 *
 * @return  EKF solution status bitfield.
 */
static inline uint16_t mavlink_msg_cc_telemetry_estimator_get_solution_status_flags(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  38);
}

/**
 * @brief Get field velocity_test_ratio from cc_telemetry_estimator message
 *
 * @return  Velocity innovation test ratio (<1 = passing).
 */
static inline float mavlink_msg_cc_telemetry_estimator_get_velocity_test_ratio(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  16);
}

/**
 * @brief Get field position_test_ratio from cc_telemetry_estimator message
 *
 * @return  Horizontal position innovation test ratio.
 */
static inline float mavlink_msg_cc_telemetry_estimator_get_position_test_ratio(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  20);
}

/**
 * @brief Get field height_test_ratio from cc_telemetry_estimator message
 *
 * @return  Height innovation test ratio.
 */
static inline float mavlink_msg_cc_telemetry_estimator_get_height_test_ratio(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  24);
}

/**
 * @brief Get field mag_test_ratio from cc_telemetry_estimator message
 *
 * @return  Magnetometer innovation test ratio.
 */
static inline float mavlink_msg_cc_telemetry_estimator_get_mag_test_ratio(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  28);
}

/**
 * @brief Get field airspeed_test_ratio from cc_telemetry_estimator message
 *
 * @return  Airspeed innovation test ratio (NaN if not applicable).
 */
static inline float mavlink_msg_cc_telemetry_estimator_get_airspeed_test_ratio(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  32);
}

/**
 * @brief Get field schema_version from cc_telemetry_estimator message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_telemetry_estimator_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  40);
}

/**
 * @brief Decode a cc_telemetry_estimator message into a struct
 *
 * @param msg The message to decode
 * @param cc_telemetry_estimator C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_telemetry_estimator_decode(const mavlink_message_t* msg, mavlink_cc_telemetry_estimator_t* cc_telemetry_estimator)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_telemetry_estimator->fc_timestamp_us = mavlink_msg_cc_telemetry_estimator_get_fc_timestamp_us(msg);
    cc_telemetry_estimator->sequence = mavlink_msg_cc_telemetry_estimator_get_sequence(msg);
    cc_telemetry_estimator->status_flags = mavlink_msg_cc_telemetry_estimator_get_status_flags(msg);
    cc_telemetry_estimator->velocity_test_ratio = mavlink_msg_cc_telemetry_estimator_get_velocity_test_ratio(msg);
    cc_telemetry_estimator->position_test_ratio = mavlink_msg_cc_telemetry_estimator_get_position_test_ratio(msg);
    cc_telemetry_estimator->height_test_ratio = mavlink_msg_cc_telemetry_estimator_get_height_test_ratio(msg);
    cc_telemetry_estimator->mag_test_ratio = mavlink_msg_cc_telemetry_estimator_get_mag_test_ratio(msg);
    cc_telemetry_estimator->airspeed_test_ratio = mavlink_msg_cc_telemetry_estimator_get_airspeed_test_ratio(msg);
    cc_telemetry_estimator->innovation_check_flags = mavlink_msg_cc_telemetry_estimator_get_innovation_check_flags(msg);
    cc_telemetry_estimator->solution_status_flags = mavlink_msg_cc_telemetry_estimator_get_solution_status_flags(msg);
    cc_telemetry_estimator->schema_version = mavlink_msg_cc_telemetry_estimator_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN? msg->len : MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN;
        memset(cc_telemetry_estimator, 0, MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_LEN);
    memcpy(cc_telemetry_estimator, _MAV_PAYLOAD(msg), len);
#endif
}
