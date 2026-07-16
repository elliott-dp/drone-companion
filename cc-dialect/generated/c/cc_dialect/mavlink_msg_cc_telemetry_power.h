#pragma once
// MESSAGE CC_TELEMETRY_POWER PACKING

#define MAVLINK_MSG_ID_CC_TELEMETRY_POWER 54002


typedef struct __mavlink_cc_telemetry_power_t {
 uint64_t fc_timestamp_us; /*< [us] FC monotonic time since PX4 boot.*/
 uint32_t sequence; /*<  Per-stream monotonic counter.*/
 float voltage; /*< [V] Pack voltage.*/
 float current; /*< [A] Pack current (positive = discharge).*/
 float power; /*< [W] Instantaneous power.*/
 float consumed_mah; /*< [mAh] Consumed charge since boot.*/
 float remaining; /*<  Remaining fraction [0..1] (NaN if unknown).*/
 float temperature; /*< [degC] Battery temperature (NaN if unknown).*/
 uint8_t cell_count; /*<  Detected/configured cell count (0 if unknown).*/
 uint8_t warning; /*<  PX4 battery warning level.*/
 uint8_t connected; /*<  1 if battery telemetry valid.*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_telemetry_power_t;

#define MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN 40
#define MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN 40
#define MAVLINK_MSG_ID_54002_LEN 40
#define MAVLINK_MSG_ID_54002_MIN_LEN 40

#define MAVLINK_MSG_ID_CC_TELEMETRY_POWER_CRC 115
#define MAVLINK_MSG_ID_54002_CRC 115



#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_POWER { \
    54002, \
    "CC_TELEMETRY_POWER", \
    12, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_power_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_power_t, sequence) }, \
         { "voltage", NULL, MAVLINK_TYPE_FLOAT, 0, 12, offsetof(mavlink_cc_telemetry_power_t, voltage) }, \
         { "current", NULL, MAVLINK_TYPE_FLOAT, 0, 16, offsetof(mavlink_cc_telemetry_power_t, current) }, \
         { "power", NULL, MAVLINK_TYPE_FLOAT, 0, 20, offsetof(mavlink_cc_telemetry_power_t, power) }, \
         { "consumed_mah", NULL, MAVLINK_TYPE_FLOAT, 0, 24, offsetof(mavlink_cc_telemetry_power_t, consumed_mah) }, \
         { "remaining", NULL, MAVLINK_TYPE_FLOAT, 0, 28, offsetof(mavlink_cc_telemetry_power_t, remaining) }, \
         { "temperature", NULL, MAVLINK_TYPE_FLOAT, 0, 32, offsetof(mavlink_cc_telemetry_power_t, temperature) }, \
         { "cell_count", NULL, MAVLINK_TYPE_UINT8_T, 0, 36, offsetof(mavlink_cc_telemetry_power_t, cell_count) }, \
         { "warning", NULL, MAVLINK_TYPE_UINT8_T, 0, 37, offsetof(mavlink_cc_telemetry_power_t, warning) }, \
         { "connected", NULL, MAVLINK_TYPE_UINT8_T, 0, 38, offsetof(mavlink_cc_telemetry_power_t, connected) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 39, offsetof(mavlink_cc_telemetry_power_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_POWER { \
    "CC_TELEMETRY_POWER", \
    12, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_power_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_power_t, sequence) }, \
         { "voltage", NULL, MAVLINK_TYPE_FLOAT, 0, 12, offsetof(mavlink_cc_telemetry_power_t, voltage) }, \
         { "current", NULL, MAVLINK_TYPE_FLOAT, 0, 16, offsetof(mavlink_cc_telemetry_power_t, current) }, \
         { "power", NULL, MAVLINK_TYPE_FLOAT, 0, 20, offsetof(mavlink_cc_telemetry_power_t, power) }, \
         { "consumed_mah", NULL, MAVLINK_TYPE_FLOAT, 0, 24, offsetof(mavlink_cc_telemetry_power_t, consumed_mah) }, \
         { "remaining", NULL, MAVLINK_TYPE_FLOAT, 0, 28, offsetof(mavlink_cc_telemetry_power_t, remaining) }, \
         { "temperature", NULL, MAVLINK_TYPE_FLOAT, 0, 32, offsetof(mavlink_cc_telemetry_power_t, temperature) }, \
         { "cell_count", NULL, MAVLINK_TYPE_UINT8_T, 0, 36, offsetof(mavlink_cc_telemetry_power_t, cell_count) }, \
         { "warning", NULL, MAVLINK_TYPE_UINT8_T, 0, 37, offsetof(mavlink_cc_telemetry_power_t, warning) }, \
         { "connected", NULL, MAVLINK_TYPE_UINT8_T, 0, 38, offsetof(mavlink_cc_telemetry_power_t, connected) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 39, offsetof(mavlink_cc_telemetry_power_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_telemetry_power message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param voltage [V] Pack voltage.
 * @param current [A] Pack current (positive = discharge).
 * @param power [W] Instantaneous power.
 * @param consumed_mah [mAh] Consumed charge since boot.
 * @param remaining  Remaining fraction [0..1] (NaN if unknown).
 * @param temperature [degC] Battery temperature (NaN if unknown).
 * @param cell_count  Detected/configured cell count (0 if unknown).
 * @param warning  PX4 battery warning level.
 * @param connected  1 if battery telemetry valid.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_power_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, float voltage, float current, float power, float consumed_mah, float remaining, float temperature, uint8_t cell_count, uint8_t warning, uint8_t connected, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, voltage);
    _mav_put_float(buf, 16, current);
    _mav_put_float(buf, 20, power);
    _mav_put_float(buf, 24, consumed_mah);
    _mav_put_float(buf, 28, remaining);
    _mav_put_float(buf, 32, temperature);
    _mav_put_uint8_t(buf, 36, cell_count);
    _mav_put_uint8_t(buf, 37, warning);
    _mav_put_uint8_t(buf, 38, connected);
    _mav_put_uint8_t(buf, 39, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN);
#else
    mavlink_cc_telemetry_power_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.voltage = voltage;
    packet.current = current;
    packet.power = power;
    packet.consumed_mah = consumed_mah;
    packet.remaining = remaining;
    packet.temperature = temperature;
    packet.cell_count = cell_count;
    packet.warning = warning;
    packet.connected = connected;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_POWER;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_CRC);
}

/**
 * @brief Pack a cc_telemetry_power message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param voltage [V] Pack voltage.
 * @param current [A] Pack current (positive = discharge).
 * @param power [W] Instantaneous power.
 * @param consumed_mah [mAh] Consumed charge since boot.
 * @param remaining  Remaining fraction [0..1] (NaN if unknown).
 * @param temperature [degC] Battery temperature (NaN if unknown).
 * @param cell_count  Detected/configured cell count (0 if unknown).
 * @param warning  PX4 battery warning level.
 * @param connected  1 if battery telemetry valid.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_power_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, float voltage, float current, float power, float consumed_mah, float remaining, float temperature, uint8_t cell_count, uint8_t warning, uint8_t connected, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, voltage);
    _mav_put_float(buf, 16, current);
    _mav_put_float(buf, 20, power);
    _mav_put_float(buf, 24, consumed_mah);
    _mav_put_float(buf, 28, remaining);
    _mav_put_float(buf, 32, temperature);
    _mav_put_uint8_t(buf, 36, cell_count);
    _mav_put_uint8_t(buf, 37, warning);
    _mav_put_uint8_t(buf, 38, connected);
    _mav_put_uint8_t(buf, 39, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN);
#else
    mavlink_cc_telemetry_power_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.voltage = voltage;
    packet.current = current;
    packet.power = power;
    packet.consumed_mah = consumed_mah;
    packet.remaining = remaining;
    packet.temperature = temperature;
    packet.cell_count = cell_count;
    packet.warning = warning;
    packet.connected = connected;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_POWER;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN);
#endif
}

/**
 * @brief Pack a cc_telemetry_power message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param voltage [V] Pack voltage.
 * @param current [A] Pack current (positive = discharge).
 * @param power [W] Instantaneous power.
 * @param consumed_mah [mAh] Consumed charge since boot.
 * @param remaining  Remaining fraction [0..1] (NaN if unknown).
 * @param temperature [degC] Battery temperature (NaN if unknown).
 * @param cell_count  Detected/configured cell count (0 if unknown).
 * @param warning  PX4 battery warning level.
 * @param connected  1 if battery telemetry valid.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_power_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t fc_timestamp_us,uint32_t sequence,float voltage,float current,float power,float consumed_mah,float remaining,float temperature,uint8_t cell_count,uint8_t warning,uint8_t connected,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, voltage);
    _mav_put_float(buf, 16, current);
    _mav_put_float(buf, 20, power);
    _mav_put_float(buf, 24, consumed_mah);
    _mav_put_float(buf, 28, remaining);
    _mav_put_float(buf, 32, temperature);
    _mav_put_uint8_t(buf, 36, cell_count);
    _mav_put_uint8_t(buf, 37, warning);
    _mav_put_uint8_t(buf, 38, connected);
    _mav_put_uint8_t(buf, 39, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN);
#else
    mavlink_cc_telemetry_power_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.voltage = voltage;
    packet.current = current;
    packet.power = power;
    packet.consumed_mah = consumed_mah;
    packet.remaining = remaining;
    packet.temperature = temperature;
    packet.cell_count = cell_count;
    packet.warning = warning;
    packet.connected = connected;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_POWER;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_CRC);
}

/**
 * @brief Encode a cc_telemetry_power struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_power C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_power_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_telemetry_power_t* cc_telemetry_power)
{
    return mavlink_msg_cc_telemetry_power_pack(system_id, component_id, msg, cc_telemetry_power->fc_timestamp_us, cc_telemetry_power->sequence, cc_telemetry_power->voltage, cc_telemetry_power->current, cc_telemetry_power->power, cc_telemetry_power->consumed_mah, cc_telemetry_power->remaining, cc_telemetry_power->temperature, cc_telemetry_power->cell_count, cc_telemetry_power->warning, cc_telemetry_power->connected, cc_telemetry_power->schema_version);
}

/**
 * @brief Encode a cc_telemetry_power struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_power C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_power_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_telemetry_power_t* cc_telemetry_power)
{
    return mavlink_msg_cc_telemetry_power_pack_chan(system_id, component_id, chan, msg, cc_telemetry_power->fc_timestamp_us, cc_telemetry_power->sequence, cc_telemetry_power->voltage, cc_telemetry_power->current, cc_telemetry_power->power, cc_telemetry_power->consumed_mah, cc_telemetry_power->remaining, cc_telemetry_power->temperature, cc_telemetry_power->cell_count, cc_telemetry_power->warning, cc_telemetry_power->connected, cc_telemetry_power->schema_version);
}

/**
 * @brief Encode a cc_telemetry_power struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_power C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_power_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_telemetry_power_t* cc_telemetry_power)
{
    return mavlink_msg_cc_telemetry_power_pack_status(system_id, component_id, _status, msg,  cc_telemetry_power->fc_timestamp_us, cc_telemetry_power->sequence, cc_telemetry_power->voltage, cc_telemetry_power->current, cc_telemetry_power->power, cc_telemetry_power->consumed_mah, cc_telemetry_power->remaining, cc_telemetry_power->temperature, cc_telemetry_power->cell_count, cc_telemetry_power->warning, cc_telemetry_power->connected, cc_telemetry_power->schema_version);
}

/**
 * @brief Send a cc_telemetry_power message
 * @param chan MAVLink channel to send the message
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param voltage [V] Pack voltage.
 * @param current [A] Pack current (positive = discharge).
 * @param power [W] Instantaneous power.
 * @param consumed_mah [mAh] Consumed charge since boot.
 * @param remaining  Remaining fraction [0..1] (NaN if unknown).
 * @param temperature [degC] Battery temperature (NaN if unknown).
 * @param cell_count  Detected/configured cell count (0 if unknown).
 * @param warning  PX4 battery warning level.
 * @param connected  1 if battery telemetry valid.
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_telemetry_power_send(mavlink_channel_t chan, uint64_t fc_timestamp_us, uint32_t sequence, float voltage, float current, float power, float consumed_mah, float remaining, float temperature, uint8_t cell_count, uint8_t warning, uint8_t connected, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, voltage);
    _mav_put_float(buf, 16, current);
    _mav_put_float(buf, 20, power);
    _mav_put_float(buf, 24, consumed_mah);
    _mav_put_float(buf, 28, remaining);
    _mav_put_float(buf, 32, temperature);
    _mav_put_uint8_t(buf, 36, cell_count);
    _mav_put_uint8_t(buf, 37, warning);
    _mav_put_uint8_t(buf, 38, connected);
    _mav_put_uint8_t(buf, 39, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_POWER, buf, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_CRC);
#else
    mavlink_cc_telemetry_power_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.voltage = voltage;
    packet.current = current;
    packet.power = power;
    packet.consumed_mah = consumed_mah;
    packet.remaining = remaining;
    packet.temperature = temperature;
    packet.cell_count = cell_count;
    packet.warning = warning;
    packet.connected = connected;
    packet.schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_POWER, (const char *)&packet, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_CRC);
#endif
}

/**
 * @brief Send a cc_telemetry_power message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_telemetry_power_send_struct(mavlink_channel_t chan, const mavlink_cc_telemetry_power_t* cc_telemetry_power)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_telemetry_power_send(chan, cc_telemetry_power->fc_timestamp_us, cc_telemetry_power->sequence, cc_telemetry_power->voltage, cc_telemetry_power->current, cc_telemetry_power->power, cc_telemetry_power->consumed_mah, cc_telemetry_power->remaining, cc_telemetry_power->temperature, cc_telemetry_power->cell_count, cc_telemetry_power->warning, cc_telemetry_power->connected, cc_telemetry_power->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_POWER, (const char *)cc_telemetry_power, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_telemetry_power_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t fc_timestamp_us, uint32_t sequence, float voltage, float current, float power, float consumed_mah, float remaining, float temperature, uint8_t cell_count, uint8_t warning, uint8_t connected, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, voltage);
    _mav_put_float(buf, 16, current);
    _mav_put_float(buf, 20, power);
    _mav_put_float(buf, 24, consumed_mah);
    _mav_put_float(buf, 28, remaining);
    _mav_put_float(buf, 32, temperature);
    _mav_put_uint8_t(buf, 36, cell_count);
    _mav_put_uint8_t(buf, 37, warning);
    _mav_put_uint8_t(buf, 38, connected);
    _mav_put_uint8_t(buf, 39, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_POWER, buf, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_CRC);
#else
    mavlink_cc_telemetry_power_t *packet = (mavlink_cc_telemetry_power_t *)msgbuf;
    packet->fc_timestamp_us = fc_timestamp_us;
    packet->sequence = sequence;
    packet->voltage = voltage;
    packet->current = current;
    packet->power = power;
    packet->consumed_mah = consumed_mah;
    packet->remaining = remaining;
    packet->temperature = temperature;
    packet->cell_count = cell_count;
    packet->warning = warning;
    packet->connected = connected;
    packet->schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_POWER, (const char *)packet, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_TELEMETRY_POWER UNPACKING


/**
 * @brief Get field fc_timestamp_us from cc_telemetry_power message
 *
 * @return [us] FC monotonic time since PX4 boot.
 */
static inline uint64_t mavlink_msg_cc_telemetry_power_get_fc_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_telemetry_power message
 *
 * @return  Per-stream monotonic counter.
 */
static inline uint32_t mavlink_msg_cc_telemetry_power_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field voltage from cc_telemetry_power message
 *
 * @return [V] Pack voltage.
 */
static inline float mavlink_msg_cc_telemetry_power_get_voltage(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  12);
}

/**
 * @brief Get field current from cc_telemetry_power message
 *
 * @return [A] Pack current (positive = discharge).
 */
static inline float mavlink_msg_cc_telemetry_power_get_current(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  16);
}

/**
 * @brief Get field power from cc_telemetry_power message
 *
 * @return [W] Instantaneous power.
 */
static inline float mavlink_msg_cc_telemetry_power_get_power(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  20);
}

/**
 * @brief Get field consumed_mah from cc_telemetry_power message
 *
 * @return [mAh] Consumed charge since boot.
 */
static inline float mavlink_msg_cc_telemetry_power_get_consumed_mah(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  24);
}

/**
 * @brief Get field remaining from cc_telemetry_power message
 *
 * @return  Remaining fraction [0..1] (NaN if unknown).
 */
static inline float mavlink_msg_cc_telemetry_power_get_remaining(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  28);
}

/**
 * @brief Get field temperature from cc_telemetry_power message
 *
 * @return [degC] Battery temperature (NaN if unknown).
 */
static inline float mavlink_msg_cc_telemetry_power_get_temperature(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  32);
}

/**
 * @brief Get field cell_count from cc_telemetry_power message
 *
 * @return  Detected/configured cell count (0 if unknown).
 */
static inline uint8_t mavlink_msg_cc_telemetry_power_get_cell_count(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  36);
}

/**
 * @brief Get field warning from cc_telemetry_power message
 *
 * @return  PX4 battery warning level.
 */
static inline uint8_t mavlink_msg_cc_telemetry_power_get_warning(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  37);
}

/**
 * @brief Get field connected from cc_telemetry_power message
 *
 * @return  1 if battery telemetry valid.
 */
static inline uint8_t mavlink_msg_cc_telemetry_power_get_connected(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  38);
}

/**
 * @brief Get field schema_version from cc_telemetry_power message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_telemetry_power_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  39);
}

/**
 * @brief Decode a cc_telemetry_power message into a struct
 *
 * @param msg The message to decode
 * @param cc_telemetry_power C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_telemetry_power_decode(const mavlink_message_t* msg, mavlink_cc_telemetry_power_t* cc_telemetry_power)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_telemetry_power->fc_timestamp_us = mavlink_msg_cc_telemetry_power_get_fc_timestamp_us(msg);
    cc_telemetry_power->sequence = mavlink_msg_cc_telemetry_power_get_sequence(msg);
    cc_telemetry_power->voltage = mavlink_msg_cc_telemetry_power_get_voltage(msg);
    cc_telemetry_power->current = mavlink_msg_cc_telemetry_power_get_current(msg);
    cc_telemetry_power->power = mavlink_msg_cc_telemetry_power_get_power(msg);
    cc_telemetry_power->consumed_mah = mavlink_msg_cc_telemetry_power_get_consumed_mah(msg);
    cc_telemetry_power->remaining = mavlink_msg_cc_telemetry_power_get_remaining(msg);
    cc_telemetry_power->temperature = mavlink_msg_cc_telemetry_power_get_temperature(msg);
    cc_telemetry_power->cell_count = mavlink_msg_cc_telemetry_power_get_cell_count(msg);
    cc_telemetry_power->warning = mavlink_msg_cc_telemetry_power_get_warning(msg);
    cc_telemetry_power->connected = mavlink_msg_cc_telemetry_power_get_connected(msg);
    cc_telemetry_power->schema_version = mavlink_msg_cc_telemetry_power_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN? msg->len : MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN;
        memset(cc_telemetry_power, 0, MAVLINK_MSG_ID_CC_TELEMETRY_POWER_LEN);
    memcpy(cc_telemetry_power, _MAV_PAYLOAD(msg), len);
#endif
}
