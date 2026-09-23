import 'dart:convert';

Map<String, dynamic> parseNewDynamoItem(
  String source, {
  required String? partitionKey,
  String? sortKey,
}) {
  if (partitionKey == null || partitionKey.trim().isEmpty) {
    throw const FormatException('Table partition key metadata is unavailable.');
  }

  final decoded = jsonDecode(source);
  if (decoded is! Map<String, dynamic>) {
    throw const FormatException('Item must be a JSON object.');
  }
  if (!decoded.containsKey(partitionKey)) {
    throw FormatException('Required partition key "$partitionKey" is missing.');
  }
  if (sortKey != null && !decoded.containsKey(sortKey)) {
    throw FormatException('Required sort key "$sortKey" is missing.');
  }
  return decoded;
}
