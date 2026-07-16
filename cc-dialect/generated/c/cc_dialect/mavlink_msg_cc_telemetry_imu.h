#pragma once
// MESSAGE CC_TELEMETRY_IMU PACKING

#define MAVLINK_MSG_ID_CC_TELEMETRY_IMU 54001


typedef struct __mavlink_cc_telemetry_imu_t {
 uint64_t fc_timestamp_us; /*< [us] FC monotonic time since PX4 boot.*/
 uint32_t sequence; /*<  Per-stream monotonic counter.*/
 uint32_t clipping_count; /*<  Cumulative accel clipping events since boot.*/
 float accel[3]; /*< [m/s/s] Filtered specific force (x, y, z).*/
 float gyro[3]; /*< [rad/s] Filtered angular rate (x, y, z).*/
 float delta_angle[3]; /*< [rad] Integrated delta angle over the sample interval.*/
 float delta_velocity[3]; /*< [m/s] Integrated delta velocity over the sample interval.*/
 float vibration_metric[3]; /*<  Per-axis vibration metric (PX4 accel vibration levels).*/
 float temperature; /*< [degC] IMU temperature.*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_telemetry_imu_t;

#define MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN 81
#define MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN 81
#define MAVLINK_MSG_ID_54001_LEN 81
#define MAVLINK_MSG_ID_54001_MIN_LEN 81

#define MAVLINK_MSG_ID_CC_TELEMETRY_IMU_CRC 203
#define MAVLINK_MSG_ID_54001_CRC 203

#define MAVLINK_MSG_CC_TELEMETRY_IMU_FIELD_ACCEL_LEN 3
#define MAVLINK_MSG_CC_TELEMETRY_IMU_FIELD_GYRO_LEN 3
#define MAVLINK_MSG_CC_TELEMETRY_IMU_FIELD_DELTA_ANGLE_LEN 3
#define MAVLINK_MSG_CC_TELEMETRY_IMU_FIELD_DELTA_VELOCITY_LEN 3
#define MAVLINK_MSG_CC_TELEMETRY_IMU_FIELD_VIBRATION_METRIC_LEN 3

#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_IMU { \
    54001, \
    "CC_TELEMETRY_IMU", \
    10, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_imu_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_imu_t, sequence) }, \
         { "clipping_count", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_telemetry_imu_t, clipping_count) }, \
         { "accel", NULL, MAVLINK_TYPE_FLOAT, 3, 16, offsetof(mavlink_cc_telemetry_imu_t, accel) }, \
         { "gyro", NULL, MAVLINK_TYPE_FLOAT, 3, 28, offsetof(mavlink_cc_telemetry_imu_t, gyro) }, \
         { "delta_angle", NULL, MAVLINK_TYPE_FLOAT, 3, 40, offsetof(mavlink_cc_telemetry_imu_t, delta_angle) }, \
         { "delta_velocity", NULL, MAVLINK_TYPE_FLOAT, 3, 52, offsetof(mavlink_cc_telemetry_imu_t, delta_velocity) }, \
         { "vibration_metric", NULL, MAVLINK_TYPE_FLOAT, 3, 64, offsetof(mavlink_cc_telemetry_imu_t, vibration_metric) }, \
         { "temperature", NULL, MAVLINK_TYPE_FLOAT, 0, 76, offsetof(mavlink_cc_telemetry_imu_t, temperature) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 80, offsetof(mavlink_cc_telemetry_imu_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_IMU { \
    "CC_TELEMETRY_IMU", \
    10, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_imu_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_imu_t, sequence) }, \
         { "clipping_count", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_telemetry_imu_t, clipping_count) }, \
         { "accel", NULL, MAVLINK_TYPE_FLOAT, 3, 16, offsetof(mavlink_cc_telemetry_imu_t, accel) }, \
         { "gyro", NULL, MAVLINK_TYPE_FLOAT, 3, 28, offsetof(mavlink_cc_telemetry_imu_t, gyro) }, \
         { "delta_angle", NULL, MAVLINK_TYPE_FLOAT, 3, 40, offsetof(mavlink_cc_telemetry_imu_t, delta_angle) }, \
         { "delta_velocity", NULL, MAVLINK_TYPE_FLOAT, 3, 52, offsetof(mavlink_cc_telemetry_imu_t, delta_velocity) }, \
         { "vibration_metric", NULL, MAVLINK_TYPE_FLOAT, 3, 64, offsetof(mavlink_cc_telemetry_imu_t, vibration_metric) }, \
         { "temperature", NULL, MAVLINK_TYPE_FLOAT, 0, 76, offsetof(mavlink_cc_telemetry_imu_t, temperature) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 80, offsetof(mavlink_cc_telemetry_imu_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_telemetry_imu message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param clipping_count  Cumulative accel clipping events since boot.
 * @param accel [m/s/s] Filtered specific force (x, y, z).
 * @param gyro [rad/s] Filtered angular rate (x, y, z).
 * @param delta_angle [rad] Integrated delta angle over the sample interval.
 * @param delta_velocity [m/s] Integrated delta velocity over the sample interval.
 * @param vibration_metric  Per-axis vibration metric (PX4 accel vibration levels).
 * @param temperature [degC] IMU temperature.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, uint32_t clipping_count, const float *accel, const float *gyro, const float *delta_angle, const float *delta_velocity, const float *vibration_metric, float temperature, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, clipping_count);
    _mav_put_float(buf, 76, temperature);
    _mav_put_uint8_t(buf, 80, schema_version);
    _mav_put_float_array(buf, 16, accel, 3);
    _mav_put_float_array(buf, 28, gyro, 3);
    _mav_put_float_array(buf, 40, delta_angle, 3);
    _mav_put_float_array(buf, 52, delta_velocity, 3);
    _mav_put_float_array(buf, 64, vibration_metric, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN);
#else
    mavlink_cc_telemetry_imu_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.clipping_count = clipping_count;
    packet.temperature = temperature;
    packet.schema_version = schema_version;
    mav_array_assign_float(packet.accel, accel, 3);
    mav_array_assign_float(packet.gyro, gyro, 3);
    mav_array_assign_float(packet.delta_angle, delta_angle, 3);
    mav_array_assign_float(packet.delta_velocity, delta_velocity, 3);
    mav_array_assign_float(packet.vibration_metric, vibration_metric, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_IMU;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_CRC);
}

/**
 * @brief Pack a cc_telemetry_imu message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param clipping_count  Cumulative accel clipping events since boot.
 * @param accel [m/s/s] Filtered specific force (x, y, z).
 * @param gyro [rad/s] Filtered angular rate (x, y, z).
 * @param delta_angle [rad] Integrated delta angle over the sample interval.
 * @param delta_velocity [m/s] Integrated delta velocity over the sample interval.
 * @param vibration_metric  Per-axis vibration metric (PX4 accel vibration levels).
 * @param temperature [degC] IMU temperature.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, uint32_t clipping_count, const float *accel, const float *gyro, const float *delta_angle, const float *delta_velocity, const float *vibration_metric, float temperature, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, clipping_count);
    _mav_put_float(buf, 76, temperature);
    _mav_put_uint8_t(buf, 80, schema_version);
    _mav_put_float_array(buf, 16, accel, 3);
    _mav_put_float_array(buf, 28, gyro, 3);
    _mav_put_float_array(buf, 40, delta_angle, 3);
    _mav_put_float_array(buf, 52, delta_velocity, 3);
    _mav_put_float_array(buf, 64, vibration_metric, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN);
#else
    mavlink_cc_telemetry_imu_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.clipping_count = clipping_count;
    packet.temperature = temperature;
    packet.schema_version = schema_version;
    mav_array_memcpy(packet.accel, accel, sizeof(float)*3);
    mav_array_memcpy(packet.gyro, gyro, sizeof(float)*3);
    mav_array_memcpy(packet.delta_angle, delta_angle, sizeof(float)*3);
    mav_array_memcpy(packet.delta_velocity, delta_velocity, sizeof(float)*3);
    mav_array_memcpy(packet.vibration_metric, vibration_metric, sizeof(float)*3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_IMU;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN);
#endif
}

/**
 * @brief Pack a cc_telemetry_imu message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param clipping_count  Cumulative accel clipping events since boot.
 * @param accel [m/s/s] Filtered specific force (x, y, z).
 * @param gyro [rad/s] Filtered angular rate (x, y, z).
 * @param delta_angle [rad] Integrated delta angle over the sample interval.
 * @param delta_velocity [m/s] Integrated delta velocity over the sample interval.
 * @param vibration_metric  Per-axis vibration metric (PX4 accel vibration levels).
 * @param temperature [degC] IMU temperature.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t fc_timestamp_us,uint32_t sequence,uint32_t clipping_count,const float *accel,const float *gyro,const float *delta_angle,const float *delta_velocity,const float *vibration_metric,float temperature,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, clipping_count);
    _mav_put_float(buf, 76, temperature);
    _mav_put_uint8_t(buf, 80, schema_version);
    _mav_put_float_array(buf, 16, accel, 3);
    _mav_put_float_array(buf, 28, gyro, 3);
    _mav_put_float_array(buf, 40, delta_angle, 3);
    _mav_put_float_array(buf, 52, delta_velocity, 3);
    _mav_put_float_array(buf, 64, vibration_metric, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN);
#else
    mavlink_cc_telemetry_imu_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.clipping_count = clipping_count;
    packet.temperature = temperature;
    packet.schema_version = schema_version;
    mav_array_assign_float(packet.accel, accel, 3);
    mav_array_assign_float(packet.gyro, gyro, 3);
    mav_array_assign_float(packet.delta_angle, delta_angle, 3);
    mav_array_assign_float(packet.delta_velocity, delta_velocity, 3);
    mav_array_assign_float(packet.vibration_metric, vibration_metric, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_IMU;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_CRC);
}

/**
 * @brief Encode a cc_telemetry_imu struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_imu C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_telemetry_imu_t* cc_telemetry_imu)
{
    return mavlink_msg_cc_telemetry_imu_pack(system_id, component_id, msg, cc_telemetry_imu->fc_timestamp_us, cc_telemetry_imu->sequence, cc_telemetry_imu->clipping_count, cc_telemetry_imu->accel, cc_telemetry_imu->gyro, cc_telemetry_imu->delta_angle, cc_telemetry_imu->delta_velocity, cc_telemetry_imu->vibration_metric, cc_telemetry_imu->temperature, cc_telemetry_imu->schema_version);
}

/**
 * @brief Encode a cc_telemetry_imu struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_imu C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_telemetry_imu_t* cc_telemetry_imu)
{
    return mavlink_msg_cc_telemetry_imu_pack_chan(system_id, component_id, chan, msg, cc_telemetry_imu->fc_timestamp_us, cc_telemetry_imu->sequence, cc_telemetry_imu->clipping_count, cc_telemetry_imu->accel, cc_telemetry_imu->gyro, cc_telemetry_imu->delta_angle, cc_telemetry_imu->delta_velocity, cc_telemetry_imu->vibration_metric, cc_telemetry_imu->temperature, cc_telemetry_imu->schema_version);
}

/**
 * @brief Encode a cc_telemetry_imu struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_imu C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_telemetry_imu_t* cc_telemetry_imu)
{
    return mavlink_msg_cc_telemetry_imu_pack_status(system_id, component_id, _status, msg,  cc_telemetry_imu->fc_timestamp_us, cc_telemetry_imu->sequence, cc_telemetry_imu->clipping_count, cc_telemetry_imu->accel, cc_telemetry_imu->gyro, cc_telemetry_imu->delta_angle, cc_telemetry_imu->delta_velocity, cc_telemetry_imu->vibration_metric, cc_telemetry_imu->temperature, cc_telemetry_imu->schema_version);
}

/**
 * @brief Send a cc_telemetry_imu message
 * @param chan MAVLink channel to send the message
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param clipping_count  Cumulative accel clipping events since boot.
 * @param accel [m/s/s] Filtered specific force (x, y, z).
 * @param gyro [rad/s] Filtered angular rate (x, y, z).
 * @param delta_angle [rad] Integrated delta angle over the sample interval.
 * @param delta_velocity [m/s] Integrated delta velocity over the sample interval.
 * @param vibration_metric  Per-axis vibration metric (PX4 accel vibration levels).
 * @param temperature [degC] IMU temperature.
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_telemetry_imu_send(mavlink_channel_t chan, uint64_t fc_timestamp_us, uint32_t sequence, uint32_t clipping_count, const float *accel, const float *gyro, const float *delta_angle, const float *delta_velocity, const float *vibration_metric, float temperature, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, clipping_count);
    _mav_put_float(buf, 76, temperature);
    _mav_put_uint8_t(buf, 80, schema_version);
    _mav_put_float_array(buf, 16, accel, 3);
    _mav_put_float_array(buf, 28, gyro, 3);
    _mav_put_float_array(buf, 40, delta_angle, 3);
    _mav_put_float_array(buf, 52, delta_velocity, 3);
    _mav_put_float_array(buf, 64, vibration_metric, 3);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_IMU, buf, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_CRC);
#else
    mavlink_cc_telemetry_imu_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.clipping_count = clipping_count;
    packet.temperature = temperature;
    packet.schema_version = schema_version;
    mav_array_assign_float(packet.accel, accel, 3);
    mav_array_assign_float(packet.gyro, gyro, 3);
    mav_array_assign_float(packet.delta_angle, delta_angle, 3);
    mav_array_assign_float(packet.delta_velocity, delta_velocity, 3);
    mav_array_assign_float(packet.vibration_metric, vibration_metric, 3);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_IMU, (const char *)&packet, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_CRC);
#endif
}

/**
 * @brief Send a cc_telemetry_imu message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_telemetry_imu_send_struct(mavlink_channel_t chan, const mavlink_cc_telemetry_imu_t* cc_telemetry_imu)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_telemetry_imu_send(chan, cc_telemetry_imu->fc_timestamp_us, cc_telemetry_imu->sequence, cc_telemetry_imu->clipping_count, cc_telemetry_imu->accel, cc_telemetry_imu->gyro, cc_telemetry_imu->delta_angle, cc_telemetry_imu->delta_velocity, cc_telemetry_imu->vibration_metric, cc_telemetry_imu->temperature, cc_telemetry_imu->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_IMU, (const char *)cc_telemetry_imu, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_telemetry_imu_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t fc_timestamp_us, uint32_t sequence, uint32_t clipping_count, const float *accel, const float *gyro, const float *delta_angle, const float *delta_velocity, const float *vibration_metric, float temperature, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, clipping_count);
    _mav_put_float(buf, 76, temperature);
    _mav_put_uint8_t(buf, 80, schema_version);
    _mav_put_float_array(buf, 16, accel, 3);
    _mav_put_float_array(buf, 28, gyro, 3);
    _mav_put_float_array(buf, 40, delta_angle, 3);
    _mav_put_float_array(buf, 52, delta_velocity, 3);
    _mav_put_float_array(buf, 64, vibration_metric, 3);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_IMU, buf, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_CRC);
#else
    mavlink_cc_telemetry_imu_t *packet = (mavlink_cc_telemetry_imu_t *)msgbuf;
    packet->fc_timestamp_us = fc_timestamp_us;
    packet->sequence = sequence;
    packet->clipping_count = clipping_count;
    packet->temperature = temperature;
    packet->schema_version = schema_version;
    mav_array_assign_float(packet->accel, accel, 3);
    mav_array_assign_float(packet->gyro, gyro, 3);
    mav_array_assign_float(packet->delta_angle, delta_angle, 3);
    mav_array_assign_float(packet->delta_velocity, delta_velocity, 3);
    mav_array_assign_float(packet->vibration_metric, vibration_metric, 3);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_IMU, (const char *)packet, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_TELEMETRY_IMU UNPACKING


/**
 * @brief Get field fc_timestamp_us from cc_telemetry_imu message
 *
 * @return [us] FC monotonic time since PX4 boot.
 */
static inline uint64_t mavlink_msg_cc_telemetry_imu_get_fc_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_telemetry_imu message
 *
 * @return  Per-stream monotonic counter.
 */
static inline uint32_t mavlink_msg_cc_telemetry_imu_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field clipping_count from cc_telemetry_imu message
 *
 * @return  Cumulative accel clipping events since boot.
 */
static inline uint32_t mavlink_msg_cc_telemetry_imu_get_clipping_count(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  12);
}

/**
 * @brief Get field accel from cc_telemetry_imu message
 *
 * @return [m/s/s] Filtered specific force (x, y, z).
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_get_accel(const mavlink_message_t* msg, float *accel)
{
    return _MAV_RETURN_float_array(msg, accel, 3,  16);
}

/**
 * @brief Get field gyro from cc_telemetry_imu message
 *
 * @return [rad/s] Filtered angular rate (x, y, z).
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_get_gyro(const mavlink_message_t* msg, float *gyro)
{
    return _MAV_RETURN_float_array(msg, gyro, 3,  28);
}

/**
 * @brief Get field delta_angle from cc_telemetry_imu message
 *
 * @return [rad] Integrated delta angle over the sample interval.
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_get_delta_angle(const mavlink_message_t* msg, float *delta_angle)
{
    return _MAV_RETURN_float_array(msg, delta_angle, 3,  40);
}

/**
 * @brief Get field delta_velocity from cc_telemetry_imu message
 *
 * @return [m/s] Integrated delta velocity over the sample interval.
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_get_delta_velocity(const mavlink_message_t* msg, float *delta_velocity)
{
    return _MAV_RETURN_float_array(msg, delta_velocity, 3,  52);
}

/**
 * @brief Get field vibration_metric from cc_telemetry_imu message
 *
 * @return  Per-axis vibration metric (PX4 accel vibration levels).
 */
static inline uint16_t mavlink_msg_cc_telemetry_imu_get_vibration_metric(const mavlink_message_t* msg, float *vibration_metric)
{
    return _MAV_RETURN_float_array(msg, vibration_metric, 3,  64);
}

/**
 * @brief Get field temperature from cc_telemetry_imu message
 *
 * @return [degC] IMU temperature.
 */
static inline float mavlink_msg_cc_telemetry_imu_get_temperature(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  76);
}

/**
 * @brief Get field schema_version from cc_telemetry_imu message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_telemetry_imu_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  80);
}

/**
 * @brief Decode a cc_telemetry_imu message into a struct
 *
 * @param msg The message to decode
 * @param cc_telemetry_imu C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_telemetry_imu_decode(const mavlink_message_t* msg, mavlink_cc_telemetry_imu_t* cc_telemetry_imu)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_telemetry_imu->fc_timestamp_us = mavlink_msg_cc_telemetry_imu_get_fc_timestamp_us(msg);
    cc_telemetry_imu->sequence = mavlink_msg_cc_telemetry_imu_get_sequence(msg);
    cc_telemetry_imu->clipping_count = mavlink_msg_cc_telemetry_imu_get_clipping_count(msg);
    mavlink_msg_cc_telemetry_imu_get_accel(msg, cc_telemetry_imu->accel);
    mavlink_msg_cc_telemetry_imu_get_gyro(msg, cc_telemetry_imu->gyro);
    mavlink_msg_cc_telemetry_imu_get_delta_angle(msg, cc_telemetry_imu->delta_angle);
    mavlink_msg_cc_telemetry_imu_get_delta_velocity(msg, cc_telemetry_imu->delta_velocity);
    mavlink_msg_cc_telemetry_imu_get_vibration_metric(msg, cc_telemetry_imu->vibration_metric);
    cc_telemetry_imu->temperature = mavlink_msg_cc_telemetry_imu_get_temperature(msg);
    cc_telemetry_imu->schema_version = mavlink_msg_cc_telemetry_imu_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN? msg->len : MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN;
        memset(cc_telemetry_imu, 0, MAVLINK_MSG_ID_CC_TELEMETRY_IMU_LEN);
    memcpy(cc_telemetry_imu, _MAV_PAYLOAD(msg), len);
#endif
}
