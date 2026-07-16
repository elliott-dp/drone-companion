#pragma once
// MESSAGE CC_MISSION_CONTEXT PACKING

#define MAVLINK_MSG_ID_CC_MISSION_CONTEXT 54012


typedef struct __mavlink_cc_mission_context_t {
 uint32_t mission_id; /*<  Mission identity minted by the companion (persisted monotonic counter).*/
 uint32_t cc_boot_id; /*<  companiond process boot identity.*/
 uint32_t vehicle_id; /*<  Static vehicle identity; must match PX4-side configuration.*/
 uint32_t dialect_hash; /*<  Hash of cc_dialect.xml this binary was generated from.*/
 char sw_version[24]; /*<  Companion software version string (git describe, NUL padded).*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_mission_context_t;

#define MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN 41
#define MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN 41
#define MAVLINK_MSG_ID_54012_LEN 41
#define MAVLINK_MSG_ID_54012_MIN_LEN 41

#define MAVLINK_MSG_ID_CC_MISSION_CONTEXT_CRC 78
#define MAVLINK_MSG_ID_54012_CRC 78

#define MAVLINK_MSG_CC_MISSION_CONTEXT_FIELD_SW_VERSION_LEN 24

#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_MISSION_CONTEXT { \
    54012, \
    "CC_MISSION_CONTEXT", \
    6, \
    {  { "mission_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 0, offsetof(mavlink_cc_mission_context_t, mission_id) }, \
         { "cc_boot_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 4, offsetof(mavlink_cc_mission_context_t, cc_boot_id) }, \
         { "vehicle_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_mission_context_t, vehicle_id) }, \
         { "dialect_hash", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_mission_context_t, dialect_hash) }, \
         { "sw_version", NULL, MAVLINK_TYPE_CHAR, 24, 16, offsetof(mavlink_cc_mission_context_t, sw_version) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 40, offsetof(mavlink_cc_mission_context_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_MISSION_CONTEXT { \
    "CC_MISSION_CONTEXT", \
    6, \
    {  { "mission_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 0, offsetof(mavlink_cc_mission_context_t, mission_id) }, \
         { "cc_boot_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 4, offsetof(mavlink_cc_mission_context_t, cc_boot_id) }, \
         { "vehicle_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_mission_context_t, vehicle_id) }, \
         { "dialect_hash", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_mission_context_t, dialect_hash) }, \
         { "sw_version", NULL, MAVLINK_TYPE_CHAR, 24, 16, offsetof(mavlink_cc_mission_context_t, sw_version) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 40, offsetof(mavlink_cc_mission_context_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_mission_context message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param mission_id  Mission identity minted by the companion (persisted monotonic counter).
 * @param cc_boot_id  companiond process boot identity.
 * @param vehicle_id  Static vehicle identity; must match PX4-side configuration.
 * @param dialect_hash  Hash of cc_dialect.xml this binary was generated from.
 * @param sw_version  Companion software version string (git describe, NUL padded).
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_mission_context_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint32_t mission_id, uint32_t cc_boot_id, uint32_t vehicle_id, uint32_t dialect_hash, const char *sw_version, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN];
    _mav_put_uint32_t(buf, 0, mission_id);
    _mav_put_uint32_t(buf, 4, cc_boot_id);
    _mav_put_uint32_t(buf, 8, vehicle_id);
    _mav_put_uint32_t(buf, 12, dialect_hash);
    _mav_put_uint8_t(buf, 40, schema_version);
    _mav_put_char_array(buf, 16, sw_version, 24);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN);
#else
    mavlink_cc_mission_context_t packet;
    packet.mission_id = mission_id;
    packet.cc_boot_id = cc_boot_id;
    packet.vehicle_id = vehicle_id;
    packet.dialect_hash = dialect_hash;
    packet.schema_version = schema_version;
    mav_array_assign_char(packet.sw_version, sw_version, 24);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_MISSION_CONTEXT;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_CRC);
}

/**
 * @brief Pack a cc_mission_context message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param mission_id  Mission identity minted by the companion (persisted monotonic counter).
 * @param cc_boot_id  companiond process boot identity.
 * @param vehicle_id  Static vehicle identity; must match PX4-side configuration.
 * @param dialect_hash  Hash of cc_dialect.xml this binary was generated from.
 * @param sw_version  Companion software version string (git describe, NUL padded).
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_mission_context_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint32_t mission_id, uint32_t cc_boot_id, uint32_t vehicle_id, uint32_t dialect_hash, const char *sw_version, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN];
    _mav_put_uint32_t(buf, 0, mission_id);
    _mav_put_uint32_t(buf, 4, cc_boot_id);
    _mav_put_uint32_t(buf, 8, vehicle_id);
    _mav_put_uint32_t(buf, 12, dialect_hash);
    _mav_put_uint8_t(buf, 40, schema_version);
    _mav_put_char_array(buf, 16, sw_version, 24);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN);
#else
    mavlink_cc_mission_context_t packet;
    packet.mission_id = mission_id;
    packet.cc_boot_id = cc_boot_id;
    packet.vehicle_id = vehicle_id;
    packet.dialect_hash = dialect_hash;
    packet.schema_version = schema_version;
    mav_array_memcpy(packet.sw_version, sw_version, sizeof(char)*24);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_MISSION_CONTEXT;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN);
#endif
}

/**
 * @brief Pack a cc_mission_context message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param mission_id  Mission identity minted by the companion (persisted monotonic counter).
 * @param cc_boot_id  companiond process boot identity.
 * @param vehicle_id  Static vehicle identity; must match PX4-side configuration.
 * @param dialect_hash  Hash of cc_dialect.xml this binary was generated from.
 * @param sw_version  Companion software version string (git describe, NUL padded).
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_mission_context_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint32_t mission_id,uint32_t cc_boot_id,uint32_t vehicle_id,uint32_t dialect_hash,const char *sw_version,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN];
    _mav_put_uint32_t(buf, 0, mission_id);
    _mav_put_uint32_t(buf, 4, cc_boot_id);
    _mav_put_uint32_t(buf, 8, vehicle_id);
    _mav_put_uint32_t(buf, 12, dialect_hash);
    _mav_put_uint8_t(buf, 40, schema_version);
    _mav_put_char_array(buf, 16, sw_version, 24);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN);
#else
    mavlink_cc_mission_context_t packet;
    packet.mission_id = mission_id;
    packet.cc_boot_id = cc_boot_id;
    packet.vehicle_id = vehicle_id;
    packet.dialect_hash = dialect_hash;
    packet.schema_version = schema_version;
    mav_array_assign_char(packet.sw_version, sw_version, 24);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_MISSION_CONTEXT;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_CRC);
}

/**
 * @brief Encode a cc_mission_context struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_mission_context C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_mission_context_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_mission_context_t* cc_mission_context)
{
    return mavlink_msg_cc_mission_context_pack(system_id, component_id, msg, cc_mission_context->mission_id, cc_mission_context->cc_boot_id, cc_mission_context->vehicle_id, cc_mission_context->dialect_hash, cc_mission_context->sw_version, cc_mission_context->schema_version);
}

/**
 * @brief Encode a cc_mission_context struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_mission_context C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_mission_context_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_mission_context_t* cc_mission_context)
{
    return mavlink_msg_cc_mission_context_pack_chan(system_id, component_id, chan, msg, cc_mission_context->mission_id, cc_mission_context->cc_boot_id, cc_mission_context->vehicle_id, cc_mission_context->dialect_hash, cc_mission_context->sw_version, cc_mission_context->schema_version);
}

/**
 * @brief Encode a cc_mission_context struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_mission_context C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_mission_context_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_mission_context_t* cc_mission_context)
{
    return mavlink_msg_cc_mission_context_pack_status(system_id, component_id, _status, msg,  cc_mission_context->mission_id, cc_mission_context->cc_boot_id, cc_mission_context->vehicle_id, cc_mission_context->dialect_hash, cc_mission_context->sw_version, cc_mission_context->schema_version);
}

/**
 * @brief Send a cc_mission_context message
 * @param chan MAVLink channel to send the message
 *
 * @param mission_id  Mission identity minted by the companion (persisted monotonic counter).
 * @param cc_boot_id  companiond process boot identity.
 * @param vehicle_id  Static vehicle identity; must match PX4-side configuration.
 * @param dialect_hash  Hash of cc_dialect.xml this binary was generated from.
 * @param sw_version  Companion software version string (git describe, NUL padded).
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_mission_context_send(mavlink_channel_t chan, uint32_t mission_id, uint32_t cc_boot_id, uint32_t vehicle_id, uint32_t dialect_hash, const char *sw_version, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN];
    _mav_put_uint32_t(buf, 0, mission_id);
    _mav_put_uint32_t(buf, 4, cc_boot_id);
    _mav_put_uint32_t(buf, 8, vehicle_id);
    _mav_put_uint32_t(buf, 12, dialect_hash);
    _mav_put_uint8_t(buf, 40, schema_version);
    _mav_put_char_array(buf, 16, sw_version, 24);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_MISSION_CONTEXT, buf, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_CRC);
#else
    mavlink_cc_mission_context_t packet;
    packet.mission_id = mission_id;
    packet.cc_boot_id = cc_boot_id;
    packet.vehicle_id = vehicle_id;
    packet.dialect_hash = dialect_hash;
    packet.schema_version = schema_version;
    mav_array_assign_char(packet.sw_version, sw_version, 24);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_MISSION_CONTEXT, (const char *)&packet, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_CRC);
#endif
}

/**
 * @brief Send a cc_mission_context message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_mission_context_send_struct(mavlink_channel_t chan, const mavlink_cc_mission_context_t* cc_mission_context)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_mission_context_send(chan, cc_mission_context->mission_id, cc_mission_context->cc_boot_id, cc_mission_context->vehicle_id, cc_mission_context->dialect_hash, cc_mission_context->sw_version, cc_mission_context->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_MISSION_CONTEXT, (const char *)cc_mission_context, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_mission_context_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint32_t mission_id, uint32_t cc_boot_id, uint32_t vehicle_id, uint32_t dialect_hash, const char *sw_version, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint32_t(buf, 0, mission_id);
    _mav_put_uint32_t(buf, 4, cc_boot_id);
    _mav_put_uint32_t(buf, 8, vehicle_id);
    _mav_put_uint32_t(buf, 12, dialect_hash);
    _mav_put_uint8_t(buf, 40, schema_version);
    _mav_put_char_array(buf, 16, sw_version, 24);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_MISSION_CONTEXT, buf, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_CRC);
#else
    mavlink_cc_mission_context_t *packet = (mavlink_cc_mission_context_t *)msgbuf;
    packet->mission_id = mission_id;
    packet->cc_boot_id = cc_boot_id;
    packet->vehicle_id = vehicle_id;
    packet->dialect_hash = dialect_hash;
    packet->schema_version = schema_version;
    mav_array_assign_char(packet->sw_version, sw_version, 24);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_MISSION_CONTEXT, (const char *)packet, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_MISSION_CONTEXT UNPACKING


/**
 * @brief Get field mission_id from cc_mission_context message
 *
 * @return  Mission identity minted by the companion (persisted monotonic counter).
 */
static inline uint32_t mavlink_msg_cc_mission_context_get_mission_id(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  0);
}

/**
 * @brief Get field cc_boot_id from cc_mission_context message
 *
 * @return  companiond process boot identity.
 */
static inline uint32_t mavlink_msg_cc_mission_context_get_cc_boot_id(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  4);
}

/**
 * @brief Get field vehicle_id from cc_mission_context message
 *
 * @return  Static vehicle identity; must match PX4-side configuration.
 */
static inline uint32_t mavlink_msg_cc_mission_context_get_vehicle_id(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field dialect_hash from cc_mission_context message
 *
 * @return  Hash of cc_dialect.xml this binary was generated from.
 */
static inline uint32_t mavlink_msg_cc_mission_context_get_dialect_hash(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  12);
}

/**
 * @brief Get field sw_version from cc_mission_context message
 *
 * @return  Companion software version string (git describe, NUL padded).
 */
static inline uint16_t mavlink_msg_cc_mission_context_get_sw_version(const mavlink_message_t* msg, char *sw_version)
{
    return _MAV_RETURN_char_array(msg, sw_version, 24,  16);
}

/**
 * @brief Get field schema_version from cc_mission_context message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_mission_context_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  40);
}

/**
 * @brief Decode a cc_mission_context message into a struct
 *
 * @param msg The message to decode
 * @param cc_mission_context C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_mission_context_decode(const mavlink_message_t* msg, mavlink_cc_mission_context_t* cc_mission_context)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_mission_context->mission_id = mavlink_msg_cc_mission_context_get_mission_id(msg);
    cc_mission_context->cc_boot_id = mavlink_msg_cc_mission_context_get_cc_boot_id(msg);
    cc_mission_context->vehicle_id = mavlink_msg_cc_mission_context_get_vehicle_id(msg);
    cc_mission_context->dialect_hash = mavlink_msg_cc_mission_context_get_dialect_hash(msg);
    mavlink_msg_cc_mission_context_get_sw_version(msg, cc_mission_context->sw_version);
    cc_mission_context->schema_version = mavlink_msg_cc_mission_context_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN? msg->len : MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN;
        memset(cc_mission_context, 0, MAVLINK_MSG_ID_CC_MISSION_CONTEXT_LEN);
    memcpy(cc_mission_context, _MAV_PAYLOAD(msg), len);
#endif
}
