#pragma once
// MESSAGE CC_TELEMETRY_ACTUATOR PACKING

#define MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR 54005


typedef struct __mavlink_cc_telemetry_actuator_t {
 uint64_t fc_timestamp_us; /*< [us] FC monotonic time since PX4 boot.*/
 uint32_t sequence; /*<  Per-stream monotonic counter.*/
 float actuator_output[8]; /*<  Commanded outputs, normalized [-1..1] or [0..1] per PX4 output function.*/
 uint8_t motor_count; /*<  Number of valid entries in actuator_output.*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_telemetry_actuator_t;

#define MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN 46
#define MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN 46
#define MAVLINK_MSG_ID_54005_LEN 46
#define MAVLINK_MSG_ID_54005_MIN_LEN 46

#define MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_CRC 229
#define MAVLINK_MSG_ID_54005_CRC 229

#define MAVLINK_MSG_CC_TELEMETRY_ACTUATOR_FIELD_ACTUATOR_OUTPUT_LEN 8

#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_ACTUATOR { \
    54005, \
    "CC_TELEMETRY_ACTUATOR", \
    5, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_actuator_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_actuator_t, sequence) }, \
         { "actuator_output", NULL, MAVLINK_TYPE_FLOAT, 8, 12, offsetof(mavlink_cc_telemetry_actuator_t, actuator_output) }, \
         { "motor_count", NULL, MAVLINK_TYPE_UINT8_T, 0, 44, offsetof(mavlink_cc_telemetry_actuator_t, motor_count) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 45, offsetof(mavlink_cc_telemetry_actuator_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_ACTUATOR { \
    "CC_TELEMETRY_ACTUATOR", \
    5, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_actuator_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_actuator_t, sequence) }, \
         { "actuator_output", NULL, MAVLINK_TYPE_FLOAT, 8, 12, offsetof(mavlink_cc_telemetry_actuator_t, actuator_output) }, \
         { "motor_count", NULL, MAVLINK_TYPE_UINT8_T, 0, 44, offsetof(mavlink_cc_telemetry_actuator_t, motor_count) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 45, offsetof(mavlink_cc_telemetry_actuator_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_telemetry_actuator message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param actuator_output  Commanded outputs, normalized [-1..1] or [0..1] per PX4 output function.
 * @param motor_count  Number of valid entries in actuator_output.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_actuator_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, const float *actuator_output, uint8_t motor_count, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 44, motor_count);
    _mav_put_uint8_t(buf, 45, schema_version);
    _mav_put_float_array(buf, 12, actuator_output, 8);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN);
#else
    mavlink_cc_telemetry_actuator_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.motor_count = motor_count;
    packet.schema_version = schema_version;
    mav_array_assign_float(packet.actuator_output, actuator_output, 8);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_CRC);
}

/**
 * @brief Pack a cc_telemetry_actuator message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param actuator_output  Commanded outputs, normalized [-1..1] or [0..1] per PX4 output function.
 * @param motor_count  Number of valid entries in actuator_output.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_actuator_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, const float *actuator_output, uint8_t motor_count, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 44, motor_count);
    _mav_put_uint8_t(buf, 45, schema_version);
    _mav_put_float_array(buf, 12, actuator_output, 8);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN);
#else
    mavlink_cc_telemetry_actuator_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.motor_count = motor_count;
    packet.schema_version = schema_version;
    mav_array_memcpy(packet.actuator_output, actuator_output, sizeof(float)*8);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN);
#endif
}

/**
 * @brief Pack a cc_telemetry_actuator message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param actuator_output  Commanded outputs, normalized [-1..1] or [0..1] per PX4 output function.
 * @param motor_count  Number of valid entries in actuator_output.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_actuator_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t fc_timestamp_us,uint32_t sequence,const float *actuator_output,uint8_t motor_count,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 44, motor_count);
    _mav_put_uint8_t(buf, 45, schema_version);
    _mav_put_float_array(buf, 12, actuator_output, 8);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN);
#else
    mavlink_cc_telemetry_actuator_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.motor_count = motor_count;
    packet.schema_version = schema_version;
    mav_array_assign_float(packet.actuator_output, actuator_output, 8);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_CRC);
}

/**
 * @brief Encode a cc_telemetry_actuator struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_actuator C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_actuator_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_telemetry_actuator_t* cc_telemetry_actuator)
{
    return mavlink_msg_cc_telemetry_actuator_pack(system_id, component_id, msg, cc_telemetry_actuator->fc_timestamp_us, cc_telemetry_actuator->sequence, cc_telemetry_actuator->actuator_output, cc_telemetry_actuator->motor_count, cc_telemetry_actuator->schema_version);
}

/**
 * @brief Encode a cc_telemetry_actuator struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_actuator C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_actuator_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_telemetry_actuator_t* cc_telemetry_actuator)
{
    return mavlink_msg_cc_telemetry_actuator_pack_chan(system_id, component_id, chan, msg, cc_telemetry_actuator->fc_timestamp_us, cc_telemetry_actuator->sequence, cc_telemetry_actuator->actuator_output, cc_telemetry_actuator->motor_count, cc_telemetry_actuator->schema_version);
}

/**
 * @brief Encode a cc_telemetry_actuator struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_actuator C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_actuator_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_telemetry_actuator_t* cc_telemetry_actuator)
{
    return mavlink_msg_cc_telemetry_actuator_pack_status(system_id, component_id, _status, msg,  cc_telemetry_actuator->fc_timestamp_us, cc_telemetry_actuator->sequence, cc_telemetry_actuator->actuator_output, cc_telemetry_actuator->motor_count, cc_telemetry_actuator->schema_version);
}

/**
 * @brief Send a cc_telemetry_actuator message
 * @param chan MAVLink channel to send the message
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param actuator_output  Commanded outputs, normalized [-1..1] or [0..1] per PX4 output function.
 * @param motor_count  Number of valid entries in actuator_output.
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_telemetry_actuator_send(mavlink_channel_t chan, uint64_t fc_timestamp_us, uint32_t sequence, const float *actuator_output, uint8_t motor_count, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 44, motor_count);
    _mav_put_uint8_t(buf, 45, schema_version);
    _mav_put_float_array(buf, 12, actuator_output, 8);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR, buf, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_CRC);
#else
    mavlink_cc_telemetry_actuator_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.motor_count = motor_count;
    packet.schema_version = schema_version;
    mav_array_assign_float(packet.actuator_output, actuator_output, 8);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR, (const char *)&packet, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_CRC);
#endif
}

/**
 * @brief Send a cc_telemetry_actuator message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_telemetry_actuator_send_struct(mavlink_channel_t chan, const mavlink_cc_telemetry_actuator_t* cc_telemetry_actuator)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_telemetry_actuator_send(chan, cc_telemetry_actuator->fc_timestamp_us, cc_telemetry_actuator->sequence, cc_telemetry_actuator->actuator_output, cc_telemetry_actuator->motor_count, cc_telemetry_actuator->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR, (const char *)cc_telemetry_actuator, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_telemetry_actuator_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t fc_timestamp_us, uint32_t sequence, const float *actuator_output, uint8_t motor_count, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 44, motor_count);
    _mav_put_uint8_t(buf, 45, schema_version);
    _mav_put_float_array(buf, 12, actuator_output, 8);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR, buf, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_CRC);
#else
    mavlink_cc_telemetry_actuator_t *packet = (mavlink_cc_telemetry_actuator_t *)msgbuf;
    packet->fc_timestamp_us = fc_timestamp_us;
    packet->sequence = sequence;
    packet->motor_count = motor_count;
    packet->schema_version = schema_version;
    mav_array_assign_float(packet->actuator_output, actuator_output, 8);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR, (const char *)packet, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_TELEMETRY_ACTUATOR UNPACKING


/**
 * @brief Get field fc_timestamp_us from cc_telemetry_actuator message
 *
 * @return [us] FC monotonic time since PX4 boot.
 */
static inline uint64_t mavlink_msg_cc_telemetry_actuator_get_fc_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_telemetry_actuator message
 *
 * @return  Per-stream monotonic counter.
 */
static inline uint32_t mavlink_msg_cc_telemetry_actuator_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field actuator_output from cc_telemetry_actuator message
 *
 * @return  Commanded outputs, normalized [-1..1] or [0..1] per PX4 output function.
 */
static inline uint16_t mavlink_msg_cc_telemetry_actuator_get_actuator_output(const mavlink_message_t* msg, float *actuator_output)
{
    return _MAV_RETURN_float_array(msg, actuator_output, 8,  12);
}

/**
 * @brief Get field motor_count from cc_telemetry_actuator message
 *
 * @return  Number of valid entries in actuator_output.
 */
static inline uint8_t mavlink_msg_cc_telemetry_actuator_get_motor_count(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  44);
}

/**
 * @brief Get field schema_version from cc_telemetry_actuator message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_telemetry_actuator_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  45);
}

/**
 * @brief Decode a cc_telemetry_actuator message into a struct
 *
 * @param msg The message to decode
 * @param cc_telemetry_actuator C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_telemetry_actuator_decode(const mavlink_message_t* msg, mavlink_cc_telemetry_actuator_t* cc_telemetry_actuator)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_telemetry_actuator->fc_timestamp_us = mavlink_msg_cc_telemetry_actuator_get_fc_timestamp_us(msg);
    cc_telemetry_actuator->sequence = mavlink_msg_cc_telemetry_actuator_get_sequence(msg);
    mavlink_msg_cc_telemetry_actuator_get_actuator_output(msg, cc_telemetry_actuator->actuator_output);
    cc_telemetry_actuator->motor_count = mavlink_msg_cc_telemetry_actuator_get_motor_count(msg);
    cc_telemetry_actuator->schema_version = mavlink_msg_cc_telemetry_actuator_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN? msg->len : MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN;
        memset(cc_telemetry_actuator, 0, MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_LEN);
    memcpy(cc_telemetry_actuator, _MAV_PAYLOAD(msg), len);
#endif
}
